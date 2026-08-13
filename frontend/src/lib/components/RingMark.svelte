<script lang="ts">
  // The Portus mark: a thin ring broken by the harbor-mouth glyph, shared by
  // empty states and the loading spinner. Reuses the FlashPad ring motif
  // without copying the bolt.
  export let size = 48;
  export let spinning = false;
  export let color = "var(--accent)";
</script>

<svg
  class="ring-mark"
  class:spinning
  width={size}
  height={size}
  viewBox="0 0 48 48"
  fill="none"
  aria-hidden="true"
>
  <path
    d="M 24 7 A 17 17 0 1 1 8.9 32.5"
    stroke={color}
    stroke-width="4"
    stroke-linecap="round"
  />
  <rect x="21.5" y="2" width="5" height="10" rx="1.5" fill={color} />
</svg>

<style>
  .ring-mark.spinning {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
