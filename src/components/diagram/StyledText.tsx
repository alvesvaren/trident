import type { StyledSpan } from "../../types/diagram";

/** Render styled spans as SVG tspans (place inside a <text> element) */
export function StyledText({ spans }: { spans: StyledSpan[] }) {
  return spans.map((span, i) => (
    <tspan
      key={i}
      fontStyle={span.italic ? "italic" : undefined}
      style={
        span.underline || span.dashed_underline
          ? { textDecorationLine: "underline", textDecorationStyle: span.underline ? "solid" : "dashed" }
          : undefined
      }
    >
      {span.text}
    </tspan>
  ));
}
