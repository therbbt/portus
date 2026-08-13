//! RDP client (IronRDP), fully isolated from the rest of the app per the
//! architecture doc. Deliberately does **not** implement `Session` — RDP is
//! a framebuffer, not a byte stream, so it gets its own connect/event shape
//! (`RdpClient` + `RdpEvent`) and its own frontend view (a `<canvas>`,
//! rather than xterm.js) instead of being forced into the terminal model.
//!
//! Scope of this first cut: view-only. Connects, authenticates, and streams
//! decoded framebuffer updates as PNG-encoded dirty rectangles. No input
//! forwarding, no clipboard, no resize — those are natural follow-ups once
//! the picture is on screen at all.

use std::io::Write as _;
use std::net::TcpStream;
use std::time::Duration;

use base64::Engine as _;
use ironrdp::connector::{self, Credentials};
use ironrdp::pdu::gcc::KeyboardType;
use ironrdp::pdu::rdp::capability_sets::MajorPlatformType;
use ironrdp::pdu::rdp::client_info::{PerformanceFlags, TimezoneInfo};
use ironrdp::session::image::DecodedImage;
use ironrdp::session::{ActiveStageBuilder, ActiveStageOutput};
use ironrdp_blocking::Framed;
use serde::{Deserialize, Serialize};
use sspi::network_client::reqwest_network_client::ReqwestNetworkClient;
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::rustls;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RdpConnectOptions {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub domain: Option<String>,
}

fn default_port() -> u16 {
    3389
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameUpdate {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// Base64-encoded PNG of just this rectangle. Encoding+shrinking to the
    /// dirty region here (rather than shipping raw RGBA over the Tauri IPC
    /// bridge as a JSON number array, like the terminal protocols' bytes
    /// do) is what keeps a busy remote desktop from flooding the webview —
    /// a full 1920x1080 raw frame would be ~8MB of JSON per update.
    pub png_base64: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RdpEvent {
    Connected { width: u16, height: u16 },
    Frame(FrameUpdate),
    Disconnected { reason: Option<String> },
    Error { message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum RdpError {
    #[error("{0}")]
    Protocol(String),
}

enum RdpCommand {
    Shutdown,
}

pub struct RdpClient {
    cmd_tx: Option<mpsc::UnboundedSender<RdpCommand>>,
}

impl Default for RdpClient {
    fn default() -> Self {
        Self { cmd_tx: None }
    }
}

impl RdpClient {
    /// Connects and authenticates before returning — matches the other
    /// protocol crates' `start()`/`connect()` convention of surfacing setup
    /// failures directly to the caller rather than only as an event. Once
    /// this returns `Ok`, framebuffer updates stream through `events` until
    /// `shutdown()` is called or the connection drops on its own.
    pub async fn connect(&mut self, options: RdpConnectOptions, events: mpsc::UnboundedSender<RdpEvent>) -> Result<(), RdpError> {
        let (ready_tx, ready_rx) = oneshot::channel();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        self.cmd_tx = Some(cmd_tx);

        tokio::task::spawn_blocking(move || run(options, events, cmd_rx, ready_tx));

        ready_rx
            .await
            .map_err(|_| RdpError::Protocol("connection task ended unexpectedly".to_string()))?
    }

    pub fn shutdown(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(RdpCommand::Shutdown);
        }
    }
}

type UpgradedFramed = Framed<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>;

fn run(
    options: RdpConnectOptions,
    events: mpsc::UnboundedSender<RdpEvent>,
    cmd_rx: mpsc::UnboundedReceiver<RdpCommand>,
    ready_tx: oneshot::Sender<Result<(), RdpError>>,
) {
    let (connection_result, framed) = match connect(&options) {
        Ok(ok) => ok,
        Err(e) => {
            let _ = ready_tx.send(Err(RdpError::Protocol(e)));
            return;
        }
    };

    let width = connection_result.desktop_size.width;
    let height = connection_result.desktop_size.height;

    let _ = ready_tx.send(Ok(()));
    let _ = events.send(RdpEvent::Connected { width, height });

    let mut image = DecodedImage::new(ironrdp::graphics::image_processing::PixelFormat::RgbA32, width, height);

    active_stage_loop(connection_result, framed, &mut image, cmd_rx, &events);
}

fn connect(options: &RdpConnectOptions) -> Result<(connector::ConnectionResult, UpgradedFramed), String> {
    let config = build_config(options);

    let server_addr = lookup_addr(&options.host, options.port).map_err(|e| format!("lookup addr: {e}"))?;

    let tcp_stream = TcpStream::connect(server_addr).map_err(|e| format!("TCP connect: {e}"))?;
    // Read timeout doubles as the active-stage loop's poll interval for
    // noticing a shutdown request without blocking on the socket forever.
    tcp_stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| format!("set_read_timeout: {e}"))?;
    let client_addr = tcp_stream.local_addr().map_err(|e| format!("local_addr: {e}"))?;

    let mut framed = Framed::new(tcp_stream);
    let mut connector = connector::ClientConnector::new(config, client_addr);

    let should_upgrade =
        ironrdp_blocking::connect_begin(&mut framed, &mut connector).map_err(|e| format!("begin connection: {e}"))?;

    let initial_stream = framed.into_inner_no_leftover();
    let (upgraded_stream, server_public_key) =
        tls_upgrade(initial_stream, options.host.clone()).map_err(|e| format!("TLS upgrade: {e}"))?;
    let upgraded = ironrdp_blocking::mark_as_upgraded(should_upgrade, &mut connector);

    let mut upgraded_framed = Framed::new(upgraded_stream);
    let mut network_client = ReqwestNetworkClient;
    let connection_result = ironrdp_blocking::connect_finalize(
        upgraded,
        connector,
        &mut upgraded_framed,
        &mut network_client,
        options.host.clone().into(),
        server_public_key,
        None,
    )
    .map_err(|e| format!("finalize connection: {e}"))?;

    Ok((connection_result, upgraded_framed))
}

fn build_config(options: &RdpConnectOptions) -> connector::Config {
    connector::Config {
        credentials: Credentials::UsernamePassword { username: options.username.clone(), password: options.password.clone() },
        domain: options.domain.clone(),
        enable_tls: false,
        enable_credssp: true,
        keyboard_type: KeyboardType::IbmEnhanced,
        keyboard_subtype: 0,
        keyboard_layout: 0,
        keyboard_functional_keys_count: 12,
        ime_file_name: String::new(),
        dig_product_id: String::new(),
        desktop_size: connector::DesktopSize { width: 1280, height: 800 },
        bitmap: None,
        client_build: 0,
        client_name: "portus".to_owned(),
        client_dir: "C:\\Windows\\System32\\mstscax.dll".to_owned(),

        #[cfg(windows)]
        platform: MajorPlatformType::WINDOWS,
        #[cfg(target_os = "macos")]
        platform: MajorPlatformType::MACINTOSH,
        #[cfg(target_os = "linux")]
        platform: MajorPlatformType::UNIX,
        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        platform: MajorPlatformType::UNIX,

        enable_server_pointer: false,
        request_data: None,
        autologon: false,
        enable_audio_playback: false,
        compression_type: None,
        pointer_software_rendering: true,
        multitransport_flags: None,
        performance_flags: PerformanceFlags::default(),
        desktop_scale_factor: 0,
        hardware_id: None,
        license_cache: None,
        timezone_info: TimezoneInfo::default(),
        alternate_shell: String::new(),
        work_dir: String::new(),
    }
}

/// Same "don't verify, just connect" posture as the reference example —
/// good enough for a first cut, but unlike SSH's TOFU known-hosts store,
/// this doesn't pin anything: nothing here would catch a changed
/// certificate on a later connection. Worth tightening before this is
/// trusted with anything sensitive.
fn tls_upgrade(stream: TcpStream, server_name: String) -> Result<(rustls::StreamOwned<rustls::ClientConnection, TcpStream>, Vec<u8>), String> {
    let mut config = rustls::client::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(NoCertificateVerification))
        .with_no_client_auth();
    config.resumption = rustls::client::Resumption::disabled();
    let config = std::sync::Arc::new(config);

    let server_name: rustls::pki_types::ServerName<'static> =
        server_name.try_into().map_err(|e| format!("invalid server name: {e}"))?;
    let client = rustls::ClientConnection::new(config, server_name).map_err(|e| e.to_string())?;
    let mut tls_stream = rustls::StreamOwned::new(client, stream);
    tls_stream.flush().map_err(|e| e.to_string())?;

    let cert = tls_stream
        .conn
        .peer_certificates()
        .and_then(|certs| certs.first())
        .ok_or_else(|| "peer certificate is missing".to_string())?;
    let server_public_key = extract_public_key(cert)?;

    Ok((tls_stream, server_public_key))
}

fn extract_public_key(cert: &rustls::pki_types::CertificateDer<'_>) -> Result<Vec<u8>, String> {
    use x509_cert::der::Decode as _;
    let cert = x509_cert::Certificate::from_der(cert).map_err(|e| e.to_string())?;
    cert.tbs_certificate()
        .subject_public_key_info()
        .subject_public_key
        .as_bytes()
        .map(|b| b.to_owned())
        .ok_or_else(|| "subject public key BIT STRING is not aligned".to_string())
}

/// Accepts any server certificate — see the caveat on `tls_upgrade` above.
#[derive(Debug)]
struct NoCertificateVerification;

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

fn active_stage_loop(
    connection_result: connector::ConnectionResult,
    mut framed: UpgradedFramed,
    image: &mut DecodedImage,
    mut cmd_rx: mpsc::UnboundedReceiver<RdpCommand>,
    events: &mpsc::UnboundedSender<RdpEvent>,
) {
    let mut active_stage = ActiveStageBuilder {
        static_channels: connection_result.static_channels,
        user_channel_id: connection_result.user_channel_id,
        io_channel_id: connection_result.io_channel_id,
        message_channel_id: connection_result.message_channel_id,
        share_id: connection_result.share_id,
        compression_type: connection_result.compression_type,
        enable_server_pointer: connection_result.enable_server_pointer,
        pointer_software_rendering: connection_result.pointer_software_rendering,
    }
    .build();

    let mut disconnect_reason: Option<String> = None;

    'outer: loop {
        if matches!(cmd_rx.try_recv(), Ok(RdpCommand::Shutdown)) {
            break;
        }

        let (action, payload) = match framed.read_pdu() {
            Ok(ok) => ok,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => {
                disconnect_reason = Some(e.to_string());
                break;
            }
        };

        let outputs = match active_stage.process(image, action, &payload) {
            Ok(outputs) => outputs,
            Err(e) => {
                disconnect_reason = Some(e.to_string());
                break;
            }
        };

        let mut dirty: Option<ironrdp::pdu::geometry::InclusiveRectangle> = None;

        for out in outputs {
            match out {
                ActiveStageOutput::ResponseFrame(frame) => {
                    if let Err(e) = framed.write_all(&frame) {
                        disconnect_reason = Some(e.to_string());
                        break 'outer;
                    }
                }
                ActiveStageOutput::GraphicsUpdate(rect) => {
                    dirty = Some(match dirty {
                        Some(existing) => union_rect(existing, rect),
                        None => rect,
                    });
                }
                ActiveStageOutput::Terminate(reason) => {
                    disconnect_reason = Some(format!("{reason:?}"));
                    break 'outer;
                }
                _ => {}
            }
        }

        if let Some(rect) = dirty {
            match encode_region(image, &rect) {
                Ok(png_base64) => {
                    let _ = events.send(RdpEvent::Frame(FrameUpdate {
                        x: rect.left,
                        y: rect.top,
                        width: rect.right - rect.left + 1,
                        height: rect.bottom - rect.top + 1,
                        png_base64,
                    }));
                }
                Err(e) => {
                    let _ = events.send(RdpEvent::Error { message: format!("frame encode failed: {e}") });
                }
            }
        }
    }

    let _ = events.send(RdpEvent::Disconnected { reason: disconnect_reason });
}

fn union_rect(
    a: ironrdp::pdu::geometry::InclusiveRectangle,
    b: ironrdp::pdu::geometry::InclusiveRectangle,
) -> ironrdp::pdu::geometry::InclusiveRectangle {
    ironrdp::pdu::geometry::InclusiveRectangle {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

fn encode_region(image: &DecodedImage, rect: &ironrdp::pdu::geometry::InclusiveRectangle) -> Result<String, String> {
    let full_width = usize::from(image.width());
    let stride = full_width * 4;
    let left = usize::from(rect.left);
    let top = usize::from(rect.top);
    let width = usize::from(rect.right - rect.left + 1);
    let height = usize::from(rect.bottom - rect.top + 1);
    let data = image.data();

    let mut cropped = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let row_start = (top + row) * stride + left * 4;
        let row_end = row_start + width * 4;
        cropped.extend_from_slice(&data[row_start..row_end]);
    }

    let buffer: image::RgbaImage =
        image::ImageBuffer::from_raw(width as u32, height as u32, cropped).ok_or_else(|| "invalid cropped buffer".to_string())?;

    let mut png_bytes = Vec::new();
    buffer
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(base64::engine::general_purpose::STANDARD.encode(png_bytes))
}

fn lookup_addr(hostname: &str, port: u16) -> std::io::Result<std::net::SocketAddr> {
    use std::net::ToSocketAddrs as _;
    (hostname, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "socket address not found"))
}
