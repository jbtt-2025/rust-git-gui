/**
 * Color palette for DAG branch lines.
 * Each branch gets a color_index that maps to one of these colors.
 */
export const DAG_COLORS = [
  '#89b4fa', // blue
  '#a6e3a1', // green
  '#f38ba8', // red
  '#f9e2af', // yellow
  '#cba6f7', // mauve
  '#89dceb', // sky
  '#fab387', // peach
  '#94e2d5', // teal
  '#f5c2e7', // pink
  '#74c7ec', // sapphire
  '#b4befe', // lavender
  '#eba0ac', // maroon
];

export function getColorForIndex(index: number): string {
  return DAG_COLORS[index % DAG_COLORS.length];
}
