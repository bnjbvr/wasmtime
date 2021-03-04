.section __TEXT,__text,regular,pure_instructions

#define GLOBL(fnname) .globl _##fnname
#define HIDDEN(fnname) .private_extern _##fnname
#define TYPE(fnname)
#define FUNCTION(fnname) _##fnname
#define SIZE(fnname)

HIDDEN(wasmtime_macos_aarch64_unwind)
GLOBL(wasmtime_macos_aarch64_unwind)

.p2align 2
TYPE(wasmtime_macos_aarch64_unwind)
FUNCTION(wasmtime_macos_aarch64_unwind):
.cfi_startproc simple
.cfi_return_column 4
    // Define what's the CFA == the stack pointer of whatever called into here.
    // Here, it's defined as ($sp + 0x10).
    .cfi_def_cfa x31, 0x10

    // Set the pseudo-register RA_SIGN_STATE to 0. This disables pointer
    // authentication for this particular stub.
    .cfi_escape 0x16,    /* DW_CFA_val_expression */ \
        0x22,            /* pseudo-reg 34 is RA_SIGN_STATE */ \
        1,               /* the byte length of this expression */ \
        0x30             /* DW_OP_lit0 */

    .cfi_offset x4, -0x08
    .cfi_offset lr, -0x10

    blr x2

    // Unreachable, since "unwind" should longjmp to the top o the wasm call
    // stack. This should help catch unexpected behaviors.
    // Use a noticeable payload so one can grep for it in the codebase.
    udf 0x1337
    .cfi_endproc
SIZE(wasmtime_macos_aarch64_unwind)
