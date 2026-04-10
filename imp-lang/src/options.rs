use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[derive(Default)]
pub struct Options {
    #[arg(short('b'), long("break"))]
    pub b: Option<Phase>,

    #[arg(short('o'), long("out"))]
    pub outdir: Option<PathBuf>,

    pub infile: PathBuf,
}

impl Options {
    pub fn new(infile: PathBuf, outdir: PathBuf) -> Self {
        Self {
            infile,
            outdir: Some(outdir),
            ..Default::default()
        }
    }

    pub fn module_name(&self) -> String {
        format!("IMP{}", self.infile.file_stem().unwrap().to_str().unwrap())
    }

    pub fn c_path(&self) -> Option<PathBuf> {
        self.outdir.as_ref().map(|outdir| {
            outdir.join(self.module_name()).with_extension("c")
        })
    }

    pub fn h_path(&self) -> Option<PathBuf> {
        self.outdir.as_ref().map(|outdir| {
            outdir.join(self.module_name()).with_extension("h")
        })
    }

    pub fn rs_path(&self) -> Option<PathBuf> {
        self.outdir.as_ref().map(|outdir| {
            outdir.join(self.module_name()).with_extension("rs")
        })
    }
}

#[derive(ValueEnum)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Read input
    RD,
    /// Scanning/parsing
    SCP,
    /// Check type pattern
    CTP,
    /// Analyse type pattern
    ATP,
    /// Flatten
    FLT,
    /// Convert to SSA
    SSA,
    /// Type inference
    TI,
    /// Constant folding
    CF,
    /// Dead code removal
    DCR,
    /// Rename fundefs
    RNF,
    /// C header code generation
    CGH,
    /// C code generation
    CGI,
    /// Rust FFI code generation
    CGR,
}
