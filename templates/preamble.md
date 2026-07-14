<!-- self:start v0.3 -->
## self — continuous learning layer

**Dispatch** — only if ALL are true: this is the first turn of the conversation
(no prior assistant turns), you are the main agent (not a subagent), and this is
an interactive user session (not an automated/headless run). Then: launch the
`SelfLearning` and `SelfImprovement` subagents in the background NOW, do not wait
for or report their results, and proceed directly with the user's request.
Both agents self-throttle; launching them redundantly is cheap and safe.

**Boundary** — never write facts/preferences into ~/.self or its skills; facts
belong to your native memory. If the user explicitly asks you to save/formalize
something, do it wherever they say — that is your job, not the system's; do not
register or route it through ~/.self.
<!-- self:end -->
