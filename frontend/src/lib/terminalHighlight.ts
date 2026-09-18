// Highlights common log/output patterns (IPs, ports, log levels, HTTP
// methods/status codes, timestamps, URLs, `ls -l` permission strings, file
// sizes, percentages, docker/git-style hex ids) in plain terminal output
// that carries no color of its own — `docker ps`, `docker logs`, access
// logs, `ls -l`, `df -h`, etc. Applied to every incoming chunk of terminal
// output (see Terminal.svelte), so it works identically for a local shell
// and an SSH session: it never looks at where the bytes came from, only at
// the text itself.
//
// Deliberately NOT attempting `ps`/`top`'s PID or %CPU/%MEM columns: those
// are bare numbers with no distinguishing shape of their own — telling a
// PID apart from a port, a size, or any other integer requires knowing
// which column of which command's output you're looking at, which a text
// pattern can't do. Matching bare numbers as if they might be PIDs would
// color every number in every line, which is noise, not signal.
//
// Colors are plain ANSI SGR codes (30-37/90-97), not literal hex — xterm.js
// resolves those against whatever the active theme (or a user's Settings
// override) assigns to that slot, so highlights stay theme-consistent
// automatically rather than fighting it with hardcoded colors.
//
// Deliberately chunk-local, not line-buffered: holding text back to wait for
// a newline that may never come (a progress bar or spinner redrawing via
// bare \r, an interactive full-screen app like htop or vim) would stall or
// garble real-time output. The cost is that a pattern split exactly across
// two separate data events won't be recognized — rare in practice, and safe
// to miss silently rather than risk delaying live output.

interface Rule {
  /** Name of a capture group in `pattern` that is always present whenever
   * this rule's alternative is the one that matched — used to tell rules
   * apart in the shared replacer. Never optional within its own alternative. */
  detector: string;
  pattern: string;
  render(groups: Record<string, string | undefined>): string;
}

// Leading `\x1b[0m` matters when a rule's match sits inside text a program
// already styled itself (a local `ip addr`/`ip -c addr` run directly in a
// TTY commonly bolds its MAC/interface fields, e.g. "\x1b[1;34m...") —
// without it, this only ever *sets a new foreground color*, and a still-
// active bold from that outer escape survives untouched underneath it.
// Bold-plus-a-plain-color renders as that color's *bright* slot in any
// terminal that honors "bold text uses bright colors" (xterm.js does, same
// as ls --color's own bold-blue directories land on --ansi-bright-blue,
// see tokens.css) — so a MAC meant to render this module's plain
// --ansi-blue silently landed on --ansi-bright-blue instead: the folder
// color, purely because bold rode along from context this module never
// asked for. The leading reset guarantees every span this module colors
// looks the same regardless of what (if anything) already styled the text
// it's sitting inside.
function sgr(code: number | string, text: string): string {
  return `\x1b[0m\x1b[${code}m${text}\x1b[0m`;
}

// A handful of colors here can't be plain ANSI indices, because the index
// they'd naturally reuse is already spoken for by something unrelated: a
// genuine green independent of the ANSI palette's own green slots (both
// deliberately tied to --accent, see tokens.css's --highlight-green
// comment, so a bold shell prompt's hostname matches the theme — meaning
// plain ANSI code 92 no longer reliably means "green" at all), a GET
// request's own color independent of cyan (now dedicated to IPv4
// addresses alone — see the ipv4 rule below), a URL's own color
// independent of blue (now dedicated to MAC addresses — see the mac rule
// below), and IPv6's own color independent of that same cyan IPv4 slot
// (an IPv4 and an IPv6 address are different enough shapes on sight that
// sharing a color was never doing much work, and splitting the two out
// means either can be recolored without recoloring the other). Each goes
// through a 24-bit truecolor escape carrying a real RGB triple instead of
// an indexed code, so it stays exactly what it's meant to be regardless of
// what any ANSI slot currently resolves to. Terminal.svelte keeps each of
// these in sync with its own `--highlight-*` custom property (light/dark,
// live on a Settings change) via the matching `setHighlight*` function
// below — the default here is only what's visible before that first runs,
// matching each var's dark-theme default.
function dedicatedColor(defaultRgb: [number, number, number]) {
  let rgb = defaultRgb;
  return {
    set(hex: string): void {
      const m = /^#?([0-9a-fA-F]{6})$/.exec(hex.trim());
      if (!m) return;
      const n = parseInt(m[1], 16);
      rgb = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
    },
    code(): string {
      return `38;2;${rgb[0]};${rgb[1]};${rgb[2]}`;
    },
  };
}

const highlightGreen = dedicatedColor([0x4d, 0xd0, 0x4d]);
/** Called by Terminal.svelte with the current theme's `--highlight-green`
 * value whenever appearance is (re)computed. Silently ignored if `hex`
 * isn't parseable, leaving whatever was previously set (or the default). */
export const setHighlightGreen = highlightGreen.set;
const successCode = highlightGreen.code;

const highlightGet = dedicatedColor([0x2d, 0xa9, 0x90]);
/** Same as `setHighlightGreen`, for `--highlight-get`. */
export const setHighlightGet = highlightGet.set;
const getMethodCode = highlightGet.code;

const highlightUrl = dedicatedColor([0xd7, 0x6a, 0x98]);
/** Same as `setHighlightGreen`, for `--highlight-url`. */
export const setHighlightUrl = highlightUrl.set;
const urlCode = highlightUrl.code;

const highlightIpv6 = dedicatedColor([0x8c, 0x72, 0xda]);
/** Same as `setHighlightGreen`, for `--highlight-ipv6`. */
export const setHighlightIpv6 = highlightIpv6.set;
const ipv6Code = highlightIpv6.code;

function httpStatusColor(code: string): number | string {
  const n = Number(code);
  if (n < 300) return successCode(); // 2xx
  if (n < 400) return 36; // 3xx
  if (n < 500) return 33; // 4xx
  return 91; // 5xx
}

function methodColor(method: string): number | string {
  switch (method) {
    case "GET":
      return getMethodCode(); // dedicated — see the note above on why not cyan
    case "POST":
      return successCode(); // creates
    case "PUT":
      return 93; // bright yellow — replaces
    case "PATCH":
      return 95; // bright magenta — partial update
    case "DELETE":
      return 91; // bright red — destructive
    default: // HEAD, OPTIONS, CONNECT, TRACE
      return 90;
  }
}

function logLevelColor(level: string): number {
  if (level === "FATAL" || level.startsWith("CRIT")) return 91;
  if (level === "ERROR") return 31;
  if (level.startsWith("WARN")) return 93;
  if (level === "INFO") return 96;
  return 90; // DEBUG, TRACE
}

function permissionColor(type: string, rwx: string): number | string {
  if (type === "d") {
    if (rwx[7] !== "w") return 94; // ordinary directory
    // World-writable (drwxrwxrwx and friends) — worth flagging on sight,
    // but not all equally dangerous: the sticky bit (rwx[8] is "t"/"T",
    // e.g. /tmp's standard 1777) still stops anyone but the owner from
    // renaming or deleting files they don't own, so that's a much smaller
    // exposure than a world-writable dir with no sticky bit at all, where
    // anyone can. Graded like percentColor/latencyColor above rather than
    // one flat warning color.
    return rwx[8] === "t" || rwx[8] === "T" ? 93 : 91;
  }
  if (type === "l") return 96; // symlink
  if (/[xsS]/.test(rwx[2] + rwx[5] + rwx[8])) return successCode(); // executable by owner, group, or other
  return 37; // regular file
}

function percentColor(n: number): number | string {
  if (n >= 90) return 91; // danger — nearly full/maxed out
  if (n >= 70) return 93; // getting close
  return successCode();
}

function latencyColor(ms: number): number | string {
  if (ms < 50) return successCode();
  if (ms < 150) return 93;
  return 91;
}

function bgpStateColor(state: string): number | string {
  const s = state.toUpperCase();
  // Established/Full are BGP/OSPF's own "fully up" states; Active is
  // usually just "still negotiating" in the strict state-machine sense,
  // but reads as a plainly positive word on sight, so it gets the same
  // treatment here rather than the transitional yellow the *other*
  // in-progress states (Connect, OpenSent, Init, ExStart, ...) keep.
  if (s === "ESTABLISHED" || s === "FULL" || s === "ACTIVE") return successCode();
  if (s === "IDLE") return 91; // down/failed
  return 93; // transitional: Connect, OpenSent, OpenConfirm, Init, ExStart, Exchange, Loading, Attempt, 2-Way
}

// Order matters: for a given start position, the first alternative that
// matches wins, same as any regex alternation — so a URL is checked before
// the IP/port rules that would otherwise fire on an IP or port embedded
// inside it, and IPv4/IPv6-with-port are checked before the bare timestamp
// rule that could otherwise match a stray "NN:NN:NN"-shaped fragment.
const RULES: Rule[] = [
  {
    detector: "url",
    pattern: String.raw`(?<url>https?://[^\s"'<>\]]+)`,
    // Dedicated color + underline, not plain blue (34) — that slot is now
    // MAC addresses' alone (see the mac rule below), and a URL needs to
    // stay visually distinct from one regardless of what either is set to.
    render: (g) => sgr(`4;${urlCode()}`, g.url!),
  },
  {
    detector: "ipv6",
    pattern: String.raw`\[(?<ipv6>[0-9a-fA-F:]+)\](?::(?<ipv6port>\d{1,5}))?(?:/(?<ipv6prefix>\d{1,3}))?`,
    // Dedicated color, not cyan — that slot is IPv4's alone now, see the
    // note above on why IPv6 got split out of it.
    render: (g) =>
      `[${sgr(ipv6Code(), g.ipv6!)}]${g.ipv6port ? `:${sgr(35, g.ipv6port)}` : ""}${g.ipv6prefix ? `/${sgr(33, g.ipv6prefix)}` : ""}`,
  },
  {
    detector: "ipv6bare",
    // Unbracketed IPv6 — `ip addr`/`ifconfig`/`ping6` all print it this way
    // (fe80::4cdc:97ff:feb0:c70/64), unlike Docker's bracketed convention
    // above. Every alternative below requires the literal "::" compression
    // marker or all 8 groups present, and every group is hex-digits-only
    // (1-4 chars) — that combination is what keeps this from firing on
    // something like C++'s `Namespace::method`: a real namespace/method
    // name almost always contains a letter outside a-f, which fails the
    // hex-group requirement outright.
    // `\b` sits inside two of the three alternatives, not wrapping the
    // whole group — a leading "::" (e.g. loopback "::1") starts with a
    // non-word character preceded by whitespace, another non-word
    // character, so a `\b` immediately before it can never match at all.
    pattern: String.raw`(?<ipv6bare>\b(?:[0-9a-fA-F]{1,4}:){1,7}:(?:[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4})*)?|::(?:[0-9a-fA-F]{1,4}:)*[0-9a-fA-F]{1,4}|\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4})(?:/(?<ipv6bareprefix>\d{1,3}))?\b`,
    render: (g) => `${sgr(ipv6Code(), g.ipv6bare!)}${g.ipv6bareprefix ? `/${sgr(33, g.ipv6bareprefix)}` : ""}`,
  },
  {
    detector: "ipv4",
    // CIDR notation (192.168.1.0/24) — the prefix gets its own color since
    // it's a subnet size, not a port, even though both are "a number after
    // a punctuation mark right after the address."
    pattern: String.raw`(?<ipv4>(?:\d{1,3}\.){3}\d{1,3})(?::(?<ipv4port>\d{1,5}))?(?:/(?<ipv4prefix>\d{1,2}))?`,
    render: (g) =>
      `${sgr(36, g.ipv4!)}${g.ipv4port ? `:${sgr(35, g.ipv4port)}` : ""}${g.ipv4prefix ? `/${sgr(33, g.ipv4prefix)}` : ""}`,
  },
  {
    detector: "mac",
    // Before hms: a MAC address's first three octets can themselves look
    // exactly like an HH:MM:SS timestamp when they happen to be decimal
    // (12:34:56:AB:CD:EF) — checked first, so the full six-octet address is
    // claimed as one span instead of hms grabbing just the leading half.
    pattern: String.raw`\b(?<mac>[0-9a-fA-F]{2}(?::[0-9a-fA-F]{2}){5}|[0-9a-fA-F]{2}(?:-[0-9a-fA-F]{2}){5})\b`,
    // Plain blue, not bright cyan — that was too close to IP addresses'
    // own cyan to read as a different thing at a glance. Blue (34) is this
    // rule's alone now — a URL used to share it (underlined) but has its
    // own dedicated color instead, see the url rule above — and it's one
    // of the few ANSI hues in this palette independently WCAG-verified
    // against both the dark and light surface colors (see tokens.css).
    render: (g) => sgr(34, g.mac!),
  },
  {
    detector: "subnetmask",
    // "netmask 255.255.255.0" / "Mask:255.255.255.0" / "Subnet Mask: ...":
    // anchored to the preceding keyword rather than just matching any
    // dotted-quad, since a bare mask looks byte-for-byte like an IP address
    // — the keyword is the only thing that actually tells them apart. The
    // prefix (keyword + separator) is captured and echoed back verbatim so
    // its exact spacing/capitalization survives untouched.
    pattern: String.raw`(?<maskprefix>(?:(?:[Nn]et)?[Mm]ask|[Ss]ubnet\s+[Mm]ask)[:\s]+)(?<subnetmask>(?:\d{1,3}\.){3}\d{1,3})`,
    render: (g) => `${g.maskprefix}${sgr(33, g.subnetmask!)}`,
  },
  {
    detector: "iface",
    // Linux-style (eth0, ens33, wlan0, docker0, veth1234, br0) and Cisco/
    // Juniper-style (GigabitEthernet0/1, Gi0/1, Vlan100) interface names.
    // Heuristic, not exhaustive — every vendor has its own naming scheme.
    // The Linux-style group also accepts a dash-then-hex-id suffix (not
    // just digits) — a custom Docker bridge network isn't numbered like
    // the default docker0, it's named "br-<12-char network ID>"
    // (e.g. br-0ebce57322c1), which used to fail this rule's \d+
    // requirement entirely (no digit immediately follows "br") and fall
    // through to plain text, leaving only the hex suffix separately
    // caught by the hexid rule below — same color either way, but with
    // the "br-" prefix itself left uncolored, looking like the interface
    // name was only half-recognized.
    pattern: String.raw`\b(?<iface>(?:eth|ens|enp|wlan|wlp|docker|veth|br|tun|tap|bond)(?:\d+[a-zA-Z0-9.]*|-[0-9a-fA-F]+)|(?:GigabitEthernet|FastEthernet|TenGigabitEthernet|HundredGigE|Ethernet|Serial|Loopback|Vlan|Port-channel|Tunnel|Gi|Fa|Te|Eth|Se|Lo|Po)\d+(?:/\d+)*(?:\.\d+)?)\b`,
    // Bright magenta, not bright blue — that slot is dedicated to real
    // directory listings (`ls --color`, see tokens.css's --ansi-bright-blue
    // note), and interface names sharing it would've made them
    // indistinguishable from folders. Reuses hexid's color rather than
    // inventing a new one; the two rarely appear on the same line.
    render: (g) => sgr(95, g.iface!),
  },
  {
    detector: "latency",
    // ping's "time=23.4 ms" and traceroute's bare "23.4 ms" per-hop probes
    // both match — the "time=" part (if present) is just left as plain
    // text before the colored number. Graded by threshold like the
    // percentage rule: fast is fine, slow is worth noticing.
    pattern: String.raw`\b(?<latency>\d+(?:\.\d+)?)(?<latencysep>\s?)ms\b`,
    render: (g) => `${sgr(latencyColor(Number(g.latency!)), g.latency!)}${g.latencysep}ms`,
  },
  {
    detector: "path",
    // Absolute (`/etc/passwd`) or home-relative (`~/.bashrc`) paths only —
    // deliberately not bare relative paths like "some/dir", which are far
    // too easily confused with ordinary "word/word" text. A plain `\b`
    // wouldn't help here (a path starts with `/` or `~`, both non-word
    // characters, so `\b` can fail to match at all when preceded by
    // whitespace — the same class of bug the bare-IPv6 rule had). What
    // actually needs guarding against is the match starting *mid*-word: a
    // bare `/[\w.-]+/` also matches the tail end of "50/50" or "and/or",
    // so this requires whatever precedes the leading `/` or `~` to NOT be
    // a word character — whitespace, punctuation, or start-of-string are
    // all fine, another digit or letter immediately before it is not.
    // The `(?!\d{1,3}\b(?!/))` guard right after the slash exists for a
    // second, less obvious reason: this rule runs per plain-text *gap*
    // between any ANSI escapes already in the line (see
    // highlightWithinEscapedText), not over the raw line as one piece. A
    // shell that colors just an address and resets before printing a CIDR
    // suffix — real `ip addr` behavior, e.g. "\x1b[1;35m192.168.1.42\x1b
    // [0m/24" — leaves "/24" starting its own gap with nothing before it
    // *in that gap*, which looks exactly like start-of-string to the
    // `(?<!\w)` lookbehind above even though it isn't really. Without this
    // guard, a bare 1-3 digit segment there (a CIDR/prefix length, 0-128)
    // gets swallowed as a fake root path and rendered plain white instead
    // of the ipv4/ipv6 rules' own yellow — and, worse, that pre-empts
    // RESET_THEN_PREFIX below from ever seeing the "reset then /NN" shape
    // it's specifically watching for, since by the time it runs this rule
    // has already rewritten that text. A real path is essentially never
    // *only* 1-3 digits with nothing else around it, so this only excludes
    // that exact shape — multi-segment paths like "/24/x" and longer
    // numeric segments like "/2024-report.pdf" both still match fine.
    pattern: String.raw`(?<!\w)(?<path>~?/(?!\d{1,3}\b(?!/))[\w.-]+(?:/[\w.-]+)*/?)`,
    render: (g) => sgr(37, g.path!),
  },
  {
    detector: "sudo",
    pattern: String.raw`\b(?<sudo>sudo)\b`,
    render: (g) => sgr(91, g.sudo!),
  },
  {
    detector: "jsonpairstrkey",
    // Requires the *key* immediately before the colon, not just any colon
    // — an earlier version matched a bare `:\s*"..."`/`:\s*123`, which
    // fired on any ordinary "Label: value" text (ifconfig's "Bcast:x",
    // "summary: 10.0.0.3", ...), not just real JSON. A quoted key right
    // before the colon is the one shape that's actually unique to JSON.
    pattern: String.raw`"(?<jsonpairstrkey>[^"\\]*(?:\\.[^"\\]*)*)"(?<jsonpairstrsep>\s*:\s*)"(?<jsonpairstrval>[^"\\]*(?:\\.[^"\\]*)*)"`,
    render: (g) => `"${sgr(96, g.jsonpairstrkey!)}"${g.jsonpairstrsep}${sgr(successCode(), `"${g.jsonpairstrval}"`)}`,
  },
  {
    detector: "jsonpairboolkey",
    pattern: String.raw`"(?<jsonpairboolkey>[^"\\]*(?:\\.[^"\\]*)*)"(?<jsonpairboolsep>\s*:\s*)(?<jsonpairboolval>true|false|null)\b`,
    render: (g) =>
      `"${sgr(96, g.jsonpairboolkey!)}"${g.jsonpairboolsep}${sgr(g.jsonpairboolval === "true" ? successCode() : g.jsonpairboolval === "false" ? 91 : 90, g.jsonpairboolval!)}`,
  },
  {
    detector: "jsonpairnumkey",
    pattern: String.raw`"(?<jsonpairnumkey>[^"\\]*(?:\\.[^"\\]*)*)"(?<jsonpairnumsep>\s*:\s*)(?<jsonpairnumval>-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)\b`,
    render: (g) => `"${sgr(96, g.jsonpairnumkey!)}"${g.jsonpairnumsep}${sgr(93, g.jsonpairnumval!)}`,
  },
  {
    detector: "jsonkey",
    // Fallback for a key whose value isn't a simple string/bool/number/null
    // (a nested object or array) — the three rules above already handle
    // the key for every simpler case, so this only ever fires for the ones
    // they didn't. Still requires nothing but a colon after the quote —
    // the one shape a JSON *key* always has that a JSON *value* never does.
    pattern: String.raw`"(?<jsonkey>[^"\\]*(?:\\.[^"\\]*)*)"(?=\s*:)`,
    render: (g) => `"${sgr(96, g.jsonkey!)}"`,
  },
  {
    detector: "arrowport",
    pattern: String.raw`->(?<arrowport>\d{1,5})(?<arrowproto>/(?:tcp|udp))\b`,
    render: (g) => `->${sgr(35, g.arrowport!)}${g.arrowproto}`,
  },
  {
    detector: "httpstatus",
    // Access-log shaped: `"GET / HTTP/1.1" 200` — a bare `\b[1-5]\d{2}\b`
    // with no such anchor would light up any ordinary 3-digit number.
    pattern: String.raw`(?<httpver>HTTP/\d(?:\.\d)?["']?\s+)(?<httpstatus>[1-5]\d{2})\b`,
    render: (g) => `${g.httpver}${sgr(httpStatusColor(g.httpstatus!), g.httpstatus!)}`,
  },
  {
    detector: "method",
    // Anchored to a leading quote (access-log shaped: `"GET /path HTTP/1.1"`)
    // rather than a bare `\bGET\b` — an unanchored match would light up
    // "get"/"put"/"delete" wherever they appear as ordinary English words.
    pattern: String.raw`"(?<method>GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS|CONNECT|TRACE)\b`,
    render: (g) => `"${sgr(methodColor(g.method!), g.method!)}`,
  },
  {
    detector: "isots",
    pattern: String.raw`\b(?<isots>\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)\b`,
    render: (g) => sgr(90, g.isots!),
  },
  {
    detector: "date",
    // Listed after isots so a full date+time timestamp is claimed whole by
    // that rule first — this only fires for a date with no time attached:
    // a bare ISO date (2026-09-17), an access-log date (17/Sep/2026), or an
    // `ls -l`-style one (Sep 17, or Jan  1  2024 for an older file — the
    // double space for a single-digit day is real `ls` output, which
    // `\s+` already tolerates).
    pattern: String.raw`\b(?<date>\d{4}-\d{2}-\d{2}|\d{1,2}/(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/\d{4}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}(?:,?\s+\d{4})?)\b`,
    render: (g) => sgr(90, g.date!),
  },
  {
    detector: "hms",
    pattern: String.raw`\b(?<hms>\d{2}:\d{2}:\d{2})\b`,
    render: (g) => sgr(90, g.hms!),
  },
  {
    detector: "loglevel",
    // Uppercase only — matching "warning" or "info" case-insensitively
    // would light up those words constantly in ordinary prose output.
    pattern: String.raw`\b(?<loglevel>FATAL|CRIT(?:ICAL)?|ERROR|WARN(?:ING)?|INFO|DEBUG|TRACE)\b`,
    render: (g) => sgr(logLevelColor(g.loglevel!), g.loglevel!),
  },
  {
    detector: "updown",
    // Capitalized/all-caps only (docker ps's "Up 9 hours", a health-check
    // script's "UP"/"DOWN"), same reasoning as loglevel above — "up" and
    // "down" as plain lowercase words are everyday English ("back up your
    // files", "scroll down") and would light up constantly if matched.
    pattern: String.raw`\b(?<updown>Up|UP|Down|DOWN)\b`,
    render: (g) => sgr(g.updown!.startsWith("U") ? successCode() : 91, g.updown!),
  },
  {
    detector: "ciscoupdown",
    // Cisco/Juniper `show ip interface brief`-style status columns print
    // lowercase "up"/"down" ("GigabitEthernet0/1 ... up      up", or
    // "administratively down    down") — end-of-line anchored AND requires
    // 2+ spaces immediately before the word. End-of-line alone still
    // matched "please back up your files before scrolling down" (ordinary
    // single-spaced prose); real fixed-width column output pads well past
    // one space before the last field, which ordinary sentences don't.
    // `\r?$` covers a PTY's "\r\n" line endings, not just "\n".
    pattern: String.raw`(?<=\s{2})(?:administratively\s+)?(?<ciscoupdown>up|down)[ \t]*\r?$`,
    render: (g) => sgr(g.ciscoupdown === "up" ? successCode() : 91, g.ciscoupdown!),
  },
  {
    detector: "bgpstate",
    // Title-case or ALL-CAPS only, same reasoning as loglevel — "idle",
    // "active", "full", and "connect" are too common as ordinary lowercase
    // (or sentence-initial) words to match unanchored. Some false-positive
    // risk remains even so (a sentence genuinely starting "Connect to...");
    // accepted the same way loglevel/updown already do.
    pattern: String.raw`\b(?<bgpstate>Established|ESTABLISHED|Idle|IDLE|Active|ACTIVE|Connect|CONNECT|OpenSent|OPENSENT|OpenConfirm|OPENCONFIRM|Full|FULL|Attempt|ATTEMPT|Init|INIT|ExStart|EXSTART|Exchange|EXCHANGE|Loading|LOADING|2-Way|2-WAY|2WAY)\b`,
    render: (g) => sgr(bgpStateColor(g.bgpstate!), g.bgpstate!),
  },
  {
    detector: "permtype",
    // Anchored to the start of a line — `ls -l`'s permission string
    // (drwxr-xr-x, -rw-r--r--, lrwxrwxrwx) always begins one. Unanchored,
    // this would risk matching an unrelated 10-character run of r/w/x/-/d/l
    // wherever one happened to appear.
    pattern: String.raw`^(?<permtype>[dlpscbD-])(?<permrwx>[-rwxsSTt]{9})`,
    render: (g) => sgr(permissionColor(g.permtype!, g.permrwx!), g.permtype! + g.permrwx!),
  },
  {
    detector: "size",
    // No space required before the unit (`du -h`/`ls -lh`'s "4.0K"), one
    // optional space allowed ("500 MB"). Requiring the unit to end on a
    // word boundary means "5Gbps" is left alone (still more letters right
    // after "G"), not miscolored as if it were a storage size.
    pattern: String.raw`\b(?<size>\d+(?:\.\d+)?)\s?(?<sizeunit>[KMGTP]i?B?)\b`,
    // Same reasoning as iface just above — kept off the now-accent-tied
    // bright-blue slot so a file size doesn't read as a folder.
    render: (g) => sgr(95, g.size! + g.sizeunit!),
  },
  {
    detector: "pct",
    pattern: String.raw`\b(?<pct>\d{1,3})%`,
    render: (g) => `${sgr(percentColor(Number(g.pct!)), g.pct!)}%`,
  },
  {
    detector: "hexid",
    // Docker container IDs (12 or 64 lowercase hex chars) and git-style
    // short/long hashes (7-40). Requires at least one a-f letter so a
    // plain long decimal number (a timestamp, a counter) isn't mistaken
    // for one — every character in a pure-digit run is technically valid
    // hex too. Listed last: its character class is the broadest of any
    // rule here, so every more specific rule gets first claim at a given
    // position.
    pattern: String.raw`\b(?=[0-9a-f]*[a-f])(?<hexid>[0-9a-f]{7,64})\b`,
    render: (g) => sgr(95, g.hexid!),
  },
];

const COMBINED_PATTERN = new RegExp(RULES.map((r) => r.pattern).join("|"), "gm");

// Matches a CSI sequence (`\x1b[...<final byte>`), an OSC sequence
// (`\x1b]...BEL-or-ST`), or a simple two-byte ESC sequence — so real escape
// sequences already in the stream are copied through untouched rather than
// scanned for patterns (which could otherwise mangle one that happens to
// contain e.g. a colon-and-digits SGR parameter) or split mid-sequence.
const ANSI_ESCAPE_SEQUENCE = /\x1b\][^\x07]*(?:\x07|\x1b\\)|\x1b\[[0-?]*[ -/]*[@-~]|\x1b[@-Z\\-_]/g;

function highlightPlainText(segment: string): string {
  if (!segment) return segment;
  COMBINED_PATTERN.lastIndex = 0;
  return segment.replace(COMBINED_PATTERN, (...args) => {
    const groups = args[args.length - 1] as Record<string, string | undefined>;
    const rule = RULES.find((r) => groups[r.detector] !== undefined);
    return rule ? rule.render(groups) : args[0];
  });
}

/** Recolors IPs/ports/log levels/HTTP status codes/timestamps/URLs found in
 * `text`, leaving any ANSI escape sequences already present untouched. Safe
 * to call on an arbitrary chunk of terminal output — see the module note on
 * why this is chunk-local rather than line-buffered. */
// A program that colors its own output can still confuse the rules above:
// `ip addr` colors the address itself, then resets *before* printing the
// CIDR suffix ("\x1b[1;35m127.0.0.1\x1b[0m/24") — that reset splits the
// address and its prefix into two separate plain-text regions, so the
// ipv4/ipv6 rules' own optional trailing "/prefix" group never gets a
// chance to see it: by the time the scan reaches "/24", there's no
// address immediately before it in that region for the pattern to attach
// to. Rather than reworking every address rule to look across escape
// sequences, this catches the specific, narrow shape that actually causes
// it — any reset immediately followed by "/NN" — as a final pass over the
// fully-processed output.
const RESET_THEN_PREFIX = /\x1b\[0m\/(\d{1,3})\b/g;

// Another self-inflicted clash from this theme's choices, not this file's
// own rules: GNU coreutils' `ls --color` marks a world-writable, non-sticky
// directory (dircolors' OTHER_WRITABLE class — a `drwxrwxrwx` dir like the
// one permtype above now flags red) with literal SGR "34;42": blue text on
// a green background. That reads fine against a real green, but this
// theme's ANSI green slot deliberately *is* --accent, a blue (see
// tokens.css's --ansi-green note) — so "blue-on-green" here means
// blue-on-nearly-identical-blue, ~1.1:1 contrast, the text is effectively
// invisible. Swapped to the same black foreground dircolors already uses
// for the sticky variant of this exact background (STICKY_OTHER_WRITABLE,
// "30;42") — not full AA contrast against a background that's secretly
// blue, but ~3.5x better, and it reuses a color already in the theme
// rather than inventing one just for this shell-emitted case. Matches any
// SGR carrying both codes, not just this exact pair, since blue-on-this-
// green is unreadable regardless of what else rode along in the sequence.
const SGR_SEQUENCE = /\x1b\[([0-9;]*)m/;
function fixLowContrastDircolors(escapeSequence: string): string {
  const match = SGR_SEQUENCE.exec(escapeSequence);
  if (!match) return escapeSequence;
  const params = match[1].split(";");
  if (!params.includes("34") || !params.includes("42")) return escapeSequence;
  return `\x1b[${params.map((p) => (p === "34" ? "30" : p)).join(";")}m`;
}

// Any single realistic chunk is fast (low single-digit milliseconds even at
// ~16KB, per measurement) — the actual failure mode is *volume*: a `SELECT
// *` against a 51k-row table streamed a few MB through in a couple hundred
// chunks measured at ~770ms of *cumulative* main-thread time, all of it
// synchronous, all of it blocking the one UI thread portus (like any
// webview app) has — indistinguishable from the app hanging. Capping
// individual chunks wouldn't have helped; those chunks were each already
// small and each already fast. What actually matters is total throughput,
// so this tracks bytes processed in a rolling window and bypasses
// highlighting entirely — a plain passthrough, next to free — once a burst
// crosses a threshold no ordinary interactive session ever approaches,
// until the flood subsides and the window resets.
const THROUGHPUT_WINDOW_MS = 500;
// ~5.65 MB/sec is roughly what this module processes (measured against the
// 51k-row case above), so this caps worst-case blocking at ~350ms — a
// brief, noticeable hitch rather than the ~770ms+ (and climbing, for an
// even bigger dump) it was before there was any cap at all. Raised from an
// initial, more conservative 200KB once 350ms was confirmed an acceptable
// trade-off for coloring substantially more of a large dump.
const THROUGHPUT_BYTE_LIMIT = 2_000_000;
// `null`, not 0 — `performance.now()` measures time since the page (or, in
// a test runner, the process) started, not since this module was first
// used. A plain `0` sentinel meant that if the very first highlight call
// happened before 500ms of page uptime had passed, `now - 0` was still
// under the window length, the reset never fired, and the byte cap below
// silently didn't engage until whatever arbitrary moment page uptime
// happened to cross 500ms — verified by tracing exactly this: a run that
// should have capped at 2MB instead colored 3.4MB before switching off.
let throughputWindowStart: number | null = null;
let throughputWindowBytes = 0;

function exceedsThroughputBudget(byteLength: number): boolean {
  const now = performance.now();
  if (throughputWindowStart === null || now - throughputWindowStart > THROUGHPUT_WINDOW_MS) {
    throughputWindowStart = now;
    throughputWindowBytes = 0;
  }
  throughputWindowBytes += byteLength;
  return throughputWindowBytes > THROUGHPUT_BYTE_LIMIT;
}

function highlightWithinEscapedText(text: string): string {
  let result = "";
  let lastIndex = 0;
  ANSI_ESCAPE_SEQUENCE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = ANSI_ESCAPE_SEQUENCE.exec(text))) {
    result += highlightPlainText(text.slice(lastIndex, match.index));
    result += fixLowContrastDircolors(match[0]);
    lastIndex = match.index + match[0].length;
  }
  result += highlightPlainText(text.slice(lastIndex));
  return result.replace(RESET_THEN_PREFIX, (_m, prefix: string) => `\x1b[0m/${sgr(33, prefix)}`);
}

// Rebuilds a shell prompt ("user@host:path$") into portus's own consistent
// style even when the shell already colored it itself. Every shell (a
// local Fedora session's own prompt script, whatever a given SSH server's
// /etc/skel sets up) has its own opinion about how to draw this — bold or
// not, one color for both segments or two, a color at all or none — and
// reacting to each one individually is an endless game of whack-a-mole
// that leaves the same prompt looking different depending purely on which
// machine drew it. This checks the *visible* text of a line (with any
// ANSI escapes stripped out just far enough to see through them) against
// the prompt shape, and on a match, discards whatever the shell sent for
// that span entirely: accent color for a normal user, bright red for root
// (matching the common Debian/Unix "root is dangerous" convention), bold
// for the path. Anchored to line-start — a prompt is always the first
// thing on its line, which also keeps "someone@example.com:" in an email
// signature or similar from qualifying.
const PROMPT_LINE_PATTERN =
  /^(?<promptuserhost>[a-zA-Z0-9_.-]+@[a-zA-Z0-9_.-]+):(?<promptpath>[^\s#$%]*)(?<promptchar>[#$%])/;

/** Strips ANSI escapes from `line`, returning the visible text plus, for
 * every character of it, which index in `line` it came from —
 * `originalIndex[stripped.length]` is `line.length`, so a match ending at
 * the end of the stripped text maps cleanly onto the end of the original
 * one too, letting a match found in the stripped text be translated back
 * into "where does the unmatched remainder start in the real line." */
function stripAnsiWithMapping(line: string): { stripped: string; originalIndex: number[] } {
  let stripped = "";
  const originalIndex: number[] = [];
  let lastIndex = 0;
  ANSI_ESCAPE_SEQUENCE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = ANSI_ESCAPE_SEQUENCE.exec(line))) {
    for (let i = lastIndex; i < match.index; i++) {
      originalIndex.push(i);
      stripped += line[i];
    }
    lastIndex = match.index + match[0].length;
  }
  for (let i = lastIndex; i < line.length; i++) {
    originalIndex.push(i);
    stripped += line[i];
  }
  originalIndex.push(line.length);
  return { stripped, originalIndex };
}

function highlightLine(line: string): string {
  const { stripped, originalIndex } = stripAnsiWithMapping(line);
  const match = PROMPT_LINE_PATTERN.exec(stripped);
  if (match && match.index === 0 && match.groups) {
    const g = match.groups;
    const restStart = originalIndex[match[0].length] ?? line.length;
    const rest = line.slice(restStart);
    const prefix = `${sgr(g.promptchar === "#" ? 91 : 92, g.promptuserhost)}:${sgr(1, g.promptpath)}${g.promptchar}`;
    return prefix + highlightWithinEscapedText(rest);
  }
  return highlightWithinEscapedText(line);
}

// PTY output uses \r\n, bare \r (a spinner/progress-bar redraw), or bare
// \n as line endings — split on any of them, keeping the exact ending
// attached to the line before it so output reconstructs byte-for-byte,
// and still keep a final "line" with no ending at all: the common case of
// a chunk ending mid-line, e.g. a freshly-printed prompt with no input
// after it yet, which needs the same prompt-detection treatment as any
// other line despite never having a trailing newline.
const LINE_ENDING = /\r\n|\r|\n/g;

function splitLines(text: string): Array<{ content: string; ending: string }> {
  const lines: Array<{ content: string; ending: string }> = [];
  let start = 0;
  LINE_ENDING.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = LINE_ENDING.exec(text))) {
    lines.push({ content: text.slice(start, match.index), ending: match[0] });
    start = match.index + match[0].length;
  }
  if (start < text.length) {
    lines.push({ content: text.slice(start), ending: "" });
  }
  return lines;
}

export function highlightTerminalOutput(text: string): string {
  if (exceedsThroughputBudget(text.length)) return text;
  return splitLines(text)
    .map(({ content, ending }) => highlightLine(content) + ending)
    .join("");
}
