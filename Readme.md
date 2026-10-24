What this shader does

- @compute marks this function as a compute shader, not a vertex or fragment shader.
- @workgroup_size(64) groups invocations into workgroups of 64.
- global_invocation_id.x identifies which particle a particular invocation updates.
- The bounds check prevents extra invocations from accessing particles outside the buffer.
-
