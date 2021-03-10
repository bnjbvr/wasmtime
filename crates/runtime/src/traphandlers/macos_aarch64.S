.section __TEXT,__text,regular,pure_instructions

#define GLOBL(fnname) .globl _##fnname
#define HIDDEN(fnname) .private_extern _##fnname
#define TYPE(fnname)
#define FUNCTION(fnname) _##fnname
#define SIZE(fnname)

// TODO fn(top_of_stack(%x0): *mut u8)
HIDDEN(wasmtime_macos_aarch64_unwind)
GLOBL(wasmtime_macos_aarch64_unwind)

.p2align 2
TYPE(wasmtime_macos_aarch64_unwind)
FUNCTION(wasmtime_macos_aarch64_unwind):
.cfi_startproc simple
    // Define what's the CFA == the stack pointer of whatever called into here.
    // Here, it's defined as ($sp + 0x10).
    .cfi_def_cfa x31, 0x10
    ;.cfi_escape 0x0f,    /* DW_CFA_def_cfa_expression */ \
        ;4,               /* the byte length of this expression */ \
        ;0x6f,            /* DW_OP_reg31(%sp) */ \
        ;0x08, 0x10,      /* DW_OP_const1u 0x10 */ \
        ;0x22             /* DW_OP_plus */

    // Set the pseudo-register RA_SIGN_STATE to 0. This disables pointer
    // authentication for this particular stub.
    .cfi_escape 0x16,    /* DW_CFA_val_expression */ \
        0x22,            /* pseudo-reg 34 is RA_SIGN_STATE */ \
        1,               /* the byte length of this expression */ \
        0x30             /* DW_OP_lit0 */

    .cfi_offset fp, -0x08
    .cfi_offset lr, -0x10

    // Our two arguments to unwind are already in the right stack slots, unwind
    // is the third argument: call it!
    blr x2

    // Unreachable, since "unwind" should longjmp to the top of the wasm call
    // stack. This should help catch unexpected behaviors.
    // Use a noticeable payload so one can grep for it in the codebase.
    udf 0x1337
    .cfi_endproc
SIZE(wasmtime_macos_aarch64_unwind)
