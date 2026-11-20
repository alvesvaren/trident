import type { StyledSpan } from "../../types/diagram";

/** Render styled spans as SVG tspans (place inside a <text> element) */
export function StyledText({ spans }: { spans: StyledSpan[] }) {
  return spans.map((span, i) => (
    <tspan key={i} fontStyle={span.italic ? "italic" : undefined} textDecoration={span.underline ? "underline" : undefined}>
      {span.text}
    </tspan>
  ));
}
