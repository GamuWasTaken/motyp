
#let frame = int(read("motyp_frame").trim())
#let animation = sys.inputs.at("animation", default: (
  width: 1920,
  height: 1080,
  fps: 60,
))

#set page(
  width: animation.width * 1pt,
  height: animation.height * 1pt
)

#place(
  center + horizon,

  dx: calc.sin(frame / 10) * 100pt,
  dy: calc.cos(frame / 10) * 100pt,

  box(width: 50pt, height: 50pt, fill: green)
)

#if frame > 150 [#metadata("motyp_end")]
