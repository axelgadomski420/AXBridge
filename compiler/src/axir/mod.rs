//! AX Intermediate Representation — warstwa pomiędzy AST a generatorami.
pub mod build;

pub use build::{
    lower, Ir, IrApp, IrField, IrFunction, IrGlobal, IrHandler, IrModel, IrNode, IrScreen, IrStmt,
    IrStore,
};
