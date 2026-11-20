import type { TextStyle } from "../types/diagram";

/** SVG text attributes for a TextStyle */
export function textStyleProps({ italic, underline }: TextStyle) {
  return {
    fontStyle: italic ? "italic" : undefined,
    textDecoration: underline ? "underline" : undefined,
  };
}
