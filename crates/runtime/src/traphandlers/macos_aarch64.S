.section __TEXT,__text,regular,pure_instructions

#define GLOBL(fnname) .globl _##fnname
#define HIDDEN(fnname) .private_extern _##fnname
#define TYPE(fnname)
#define FUNCTION(fnname) _##fnname
#define SIZE(fnname)

HIDDEN(wasmtime_macos_aarch64_unwind)
GLOBL(wasmtime_macos_aarch64_unwind)

// This small stub helps connect the native code and the wasm call frames when
// unwinding a stack. This must be kept in close sync with macos.rs in the same
// directory.

// The signature of this function is:
// wasmtime_macos_aarch64_unwind(faulting_pc, setjmp_buf, unwind_fn)

.p2align 2
TYPE(wasmtime_macos_aarch64_unwind)
FUNCTION(wasmtime_macos_aarch64_unwind):
.cfi_startproc simple
// This directive does the magic. In general, when unwinding aarch64 call
// frames, LR == PC. But when unwinding wasm frames, the PC may be at a value
// that's strictly different from PC. So first we want to unwind the frame
// designated by PC, and then only use LR to unwind to its caller.
//
// This directive says that the return register for this particular frame is
// located in x3. Note that the choice of x3 is arbitrary among the possible
// available registers (x0, x1 and x2 are used, as function arguments). Then,
// another directive below defines where x3 is located: this matches the stack
// slot where we've put PC, in macos.rs, so we're really indicating that the
// next call frame must be found from PC.
.cfi_return_column 3
    // Define what's the CFA == the stack pointer of whatever called into here.
    // Here, it's defined as ($sp + 0x10), as we've allocated 16 bytes in the
    // resume function.
    .cfi_def_cfa x31, 0x10

    // Set the pseudo-register RA_SIGN_STATE to 0. This disables pointer
    // authentication of the return address for this particular stub.
    .cfi_escape 0x16,    /* DW_CFA_val_expression */ \
        0x22,            /* pseudo-reg 34 is RA_SIGN_STATE */ \
        1,               /* the byte length of this expression */ \
        0x30             /* DW_OP_lit0 */

    .cfi_offset x3, -0x08 // location of PC
    .cfi_offset lr, -0x10 // location of the real LR

    // The unwind function is stored in x2. Other arguments to this function
    // are already in the right places (x0 and x1). We can branch and link to
    // x2 (LR has been explicitly saved on the stack).
    blr x2

    // Unreachable, since "unwind" should longjmp to the top o the wasm call
    // stack. This should help catch unexpected behaviors.
    // Use a noticeable payload so one can grep for it in the codebase.
    udf 0x1337
    .cfi_endproc
SIZE(wasmtime_macos_aarch64_unwind)
