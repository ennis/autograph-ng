use spirv_headers::*;

pub struct RawInstruction<'m> {
    pub opcode: u16,
    pub word_count: u16,
    pub operands: &'m [u32],
}

#[derive(Debug, Clone)]
pub enum Instruction<'m> {
    Unknown(IUnknownInst),
    Nop,
    Name(IName),
    MemberName(IMemberName),
    ExtInstImport(IExtInstImport),
    MemoryModel(IMemoryModel),
    EntryPoint(IEntryPoint<'m>),
    ExecutionMode(IExecutionMode<'m>),
    Capability(ICapability),
    TypeVoid(ITypeVoid),
    TypeBool(ITypeBool),
    TypeInt(ITypeInt),
    TypeFloat(ITypeFloat),
    TypeVector(ITypeVector),
    TypeMatrix(ITypeMatrix),
    TypeImage(ITypeImage),
    TypeSampler(ITypeSampler),
    TypeSampledImage(ITypeSampledImage),
    TypeArray(ITypeArray),
    TypeRuntimeArray(ITypeRuntimeArray),
    TypeStruct(ITypeStruct<'m>),
    TypeOpaque(ITypeOpaque),
    TypePointer(ITypePointer),
    Constant(IConstant<'m>),
    FunctionEnd,
    Variable(IVariable),
    Decorate(IDecorate<'m>),
    MemberDecorate(IMemberDecorate<'m>),
    Label(ILabel),
    Branch(IBranch),
    Kill,
    Return,
}

#[derive(Debug, Clone)]
pub struct IUnknownInst(pub u16, pub Vec<u32>);

#[derive(Debug, Clone)]
pub struct IName {
    pub target_id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IMemberName {
    pub target_id: u32,
    pub member: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IExtInstImport {
    pub result_id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IMemoryModel(pub AddressingModel, pub MemoryModel);

#[derive(Debug, Clone)]
pub struct IEntryPoint<'m> {
    pub execution: ExecutionModel,
    pub id: u32,
    pub name: String,
    pub interface: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct IExecutionMode<'m> {
    pub target_id: u32,
    pub mode: ExecutionMode,
    pub optional_literals: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct ICapability(pub Capability);

#[derive(Debug, Clone)]
pub struct ITypeVoid {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeBool {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeInt {
    pub result_id: u32,
    pub width: u32,
    pub signedness: bool,
}

#[derive(Debug, Clone)]
pub struct ITypeFloat {
    pub result_id: u32,
    pub width: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeVector {
    pub result_id: u32,
    pub component_id: u32,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeMatrix {
    pub result_id: u32,
    pub column_type_id: u32,
    pub column_count: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeImage {
    pub result_id: u32,
    pub sampled_type_id: u32,
    pub dim: Dim,
    pub depth: Option<bool>,
    pub arrayed: bool,
    pub ms: bool,
    pub sampled: Option<bool>,
    pub format: ImageFormat,
    pub access: Option<AccessQualifier>,
}

#[derive(Debug, Clone)]
pub struct ITypeSampler {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeSampledImage {
    pub result_id: u32,
    pub image_type_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeArray {
    pub result_id: u32,
    pub type_id: u32,
    pub length_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeRuntimeArray {
    pub result_id: u32,
    pub type_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeStruct<'m> {
    pub result_id: u32,
    pub member_types: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct ITypeOpaque {
    pub result_id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ITypePointer {
    pub result_id: u32,
    pub storage_class: StorageClass,
    pub type_id: u32,
}

#[derive(Debug, Clone)]
pub struct IConstant<'m> {
    pub result_type_id: u32,
    pub result_id: u32,
    pub data: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct IVariable {
    pub result_type_id: u32,
    pub result_id: u32,
    pub storage_class: StorageClass,
    pub initializer: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct IDecorate<'m> {
    pub target_id: u32,
    pub decoration: Decoration,
    pub params: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct IMemberDecorate<'m> {
    pub target_id: u32,
    pub member: u32,
    pub decoration: Decoration,
    pub params: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct ILabel {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct IBranch {
    pub result_id: u32,
}
