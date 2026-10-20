# Control-flow lowering

## Terminal conditions

Lowering emits an entry block, condition evaluation, and branch, then separate
arm blocks. Both arms may read parameters and enclosing locals. The
[IR rules](../ir/control-flow.md#ir-control-flow) validate
block-local values and initialization on every path. See
[conditions.mc](../../../examples/compiler-probe/conditions.mc) and
[initial simulator evidence](../../investigations/compiler-conditions.md).

## While loops

Lowering uses explicit test/body/exit blocks and a symbolic backward jump from
continuing body paths. Conditions are evaluated in the test block each visit;
body effects stay in the body. Every edge has an empty evaluation stack. These
use branch/jump directives; cyclic initialization checks are specified in [IR
text](../ir/control-flow.md#cyclic-control-flow).

## Expression joins

Lower a value edge to its operand and `goto`; the parameter emits no load, copy,
placeholder initializer, or native call. Exactly one value arrives on the VM
stack. Consume it in a return, discard, local initialization/assignment, branch,
or another value jump. A prefix-free chain of unary operations or left operands
of binary/comparison operations may precede that consumption. Operations that
push call, field, or array setup must first stage the result in a local. Both
stack-use validation and declaration/evaluation-order validation enforce this;
no unrelated root may run before the parameter is consumed. Ordinary edges keep
an empty stack. Surrounding operand staging shares the existing frame budget; a
direct conditional return needs no additional local.
