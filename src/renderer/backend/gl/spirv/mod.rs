mod parse;
mod reflect;

pub use self::parse::Module;

// SPIR-V -> parse instructions (SpirvModule) -> generate reflection data (SpirvReflection)
