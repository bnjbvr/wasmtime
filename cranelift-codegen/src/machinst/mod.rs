//! This module exposes the machine-specific backend definition pieces.
//!
//! Machine backends define two types, an `Op` (opcode) and `Arg` (instruction argument, meant to
//! encapsulate, register, immediate, and memory references).
//!
//! Machine instructions (`MachInst` instances) are statically parameterized on these types for efficiency.
//! They in turn are held by an implementation of the `MachInsts` trait that the `Function` holds
//! by dynamic reference, in order to avoid parameterizing the entire IR world on machine backend.
//!
//! The `Function` requests a lowering of its IR (`ir::Inst`s) into machine-specific code, and the
//! results are kept alongside the original IR, with a 1-to-N correspondence: each Cranelift IR
//! instruction can correspond to N contiguous machine instructions. (N=0 is possible, if e.g. two
//! IR instructions are fused into a single machine instruction: then the final value-producing
//! instruction is the only one that has machine instructions.)
//!
//! To keep the interface with the register allocator simple, the control-flow and the register
//! defs/uses of the Cranelift IR remain mostly canonical, even after lowering. There is one
//! exception: because instruction lowering may require extra temps within a sequence of machine
//! instructions (a Value that is def'd and use'd immediately), or may use a value from an earlier
//! IR instruction if fusing instructions, we need to be able to add new args and results to
//! Cranelift IR instructions. Rather than rewrite the instructions in place, or somehow alter
//! their format, the `MachInsts` container keeps extra `Value` args and results for instructions
//! as it goes. The register allocator queries this as well as the original instruction. (Why not
//! just rewrite the whole list of defs/uses and make the regalloc ignore the originals? The common
//! case is that no defs/uses change; the "exceptions" list should in the common case be very
//! short or empty, leading to less memory overhead.)

/* TODO:

  - Top level compilation pipeline with new MachInst / VCode stuff:

    - Split critical edges
    - Machine-specific lowering
    - Regalloc (minira)
    - Binemit

*/

use crate::binemit::CodeSink;
use crate::entity::EntityRef;
use crate::entity::SecondaryMap;
use crate::ir::ValueLocations;
use crate::ir::{DataFlowGraph, Inst, Opcode, Type, Value};
use crate::isa::registers::{RegClass, RegClassMask};
use crate::isa::RegUnit;
use crate::HashMap;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::iter::Sum;
use smallvec::SmallVec;
use std::hash::Hash;

pub mod lower;
pub use lower::*;
pub mod vcode;
pub use vcode::*;
pub mod branch_splitting;
pub use branch_splitting::*;
pub mod compile;
pub use compile::*;

/// A constraint on a virtual register in a machine instruction.
#[derive(Clone, Debug)]
pub enum MachRegConstraint {
    /// Any register in one of the given register classes.
    RegClass(RegClassMask),
    /// A particular, fixed register.
    FixedReg(RegUnit),
}

impl MachRegConstraint {
    /// Create a machine-register constraint that chooses a register from a single register class
    /// (or its subclasses).
    pub fn from_class(rc: RegClass) -> MachRegConstraint {
        MachRegConstraint::RegClass(rc.subclasses)
    }
    /// Create a machine-register constraint that chooses a fixed register.
    pub fn from_fixed(ru: RegUnit) -> MachRegConstraint {
        MachRegConstraint::FixedReg(ru)
    }
}

/// A register reference in a machine instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachReg {
    /// Undefined. Used as a placeholder.
    Undefined,
    /// A virtual register.
    Virtual(usize),
    /// An allocated physical register.
    Allocated(RegUnit),
    /// The zero register.
    Zero,
}

impl MachReg {
    /// Get the zero register.
    pub fn zero() -> MachReg {
        MachReg::Zero
    }
}

/// The mode in which a register is used or defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachRegMode {
    /// Read (used) by the instruction.
    Use,
    /// Written (defined) by the instruction.
    Def,
    /// Both read and written by the instruction.
    Modify,
}

/// A list of MachRegs used/def'd by a MachInst.
pub type MachInstRegs = SmallVec<[(MachReg, MachRegMode); 4]>;

/// A list of MachRegs with associated constraints.
pub type MachInstRegConstraints = SmallVec<[(MachReg, MachRegConstraint); 4]>;

/// A machine instruction.
pub trait MachInst {
    /// Return the registers referenced by this machine instruction along with the modes of
    /// reference (use, def, modify).
    fn regs(&self) -> MachInstRegs;

    /// Return the constraints, if any, on registers in this instruction.
    fn reg_constraints(&self) -> MachInstRegConstraints;

    /// Map virtual registers to physical registers using the given virt->phys map.
    fn map_virtregs(&mut self, locs: &MachLocations);

    /// If this is a simple move, return the (source, destination) tuple of registers.
    fn is_move(&self) -> Option<(MachReg, MachReg)>;

    /// Finalize this instruction: convert any virtual instruction into a real one.
    fn finalize(&mut self);

    /// Is this a terminator (branch or ret)? If so, return its type
    /// (ret/uncond/cond) and target if applicable.
    fn is_term(&self) -> MachTerminator;
}

/// Describes a block terminator (not call) in the vcode. Because MachInsts /
/// vcode model machine code fairly directly (modulo the virtual registers), we
/// do not have a two-target conditional branch. Rather, the conditional form
/// falls through if not taken. A conditional branch should always be followed
/// by an unconditional branch; branches to the next block will be elided (to
/// allow fallthrough instead).
#[derive(Clone, Debug)]
pub enum MachTerminator {
    /// Not a terminator.
    None,
    /// A return instruction.
    Ret,
    /// An unconditional branch to another block.
    Uncond(BlockIndex),
    /// A conditional branch to one of two other blocks.
    Cond(BlockIndex, BlockIndex),
}

/// A map from virtual registers to physical registers.
pub type MachLocations = Vec<RegUnit>; // Indexed by virtual register number.

/// A trait describing the ability to encode a MachInst into binary machine code.
pub trait MachInstEmit<CS: CodeSink> {
    /// Get the size of the instruction.
    fn size(&self) -> usize;

    /// Emit the instruction.
    fn emit(&self, cs: &mut CS);
}