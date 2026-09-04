<script lang="ts">
  // The actual application icon (see app/portus/icons/portus-icon.svg) -
  // a dark tile with a ring and terminal ">" prompt/cursor cut through it.
  // Distinct from RingMark.svelte (the thin monochrome loading-spinner/
  // empty-state glyph) - this renders the real icon's own fixed colors,
  // for places that should show the actual brand mark, like the titlebar.
  export let size = 16;
</script>

<svg width={size} height={size} viewBox="0 0 48 48" aria-hidden="true">
  <rect x="2" y="2" width="44" height="44" rx="12" fill="#26262c" />
  <circle cx="24" cy="25" r="15.53" fill="none" stroke="#fafafa" stroke-width="4.6" />
  <g fill="none" stroke="#26262c" stroke-width="10.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M16.8 15.5 L27.5 25 L16.8 34.5" />
    <path d="M31.5 34.5 L40.5 34.5" />
  </g>
  <g fill="none" stroke="#fafafa" stroke-width="4.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M16.8 15.5 L27.5 25 L16.8 34.5" />
    <path d="M31.5 34.5 L40.5 34.5" />
  </g>
</svg>
