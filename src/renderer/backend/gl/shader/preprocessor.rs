use regex::Regex;
use std::error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::renderer::backend::gl::*;
use crate::renderer::format::Format;
use crate::renderer::sampler::{Filter, SamplerAddressMode, SamplerDescription, SamplerMipmapMode};
use crate::renderer::{PrimitiveTopology, ShaderStageFlags, VertexInputAttributeDescription};

pub struct SourceMapEntry {
    pub index: u32,
    pub path: Option<PathBuf>,
}

struct IncludeFile<'a> {
    parent: Option<&'a IncludeFile<'a>>,
    path: Option<&'a Path>,
}

pub struct PreprocessResult {
    pub preprocessed_source: String,
    pub stages: ShaderStageFlags,
    pub vertex_attributes: Option<Vec<VertexInputAttributeDescription>>,
    pub topology: Option<PrimitiveTopology>,
    pub source_map: Vec<SourceMapEntry>,
    pub version: Option<u32>,
    pub descriptor_map: Vec<DescriptorMapEntry>,
    pub static_samplers: Vec<StaticSamplerEntry>,
}

impl PreprocessResult {
    fn new() -> PreprocessResult {
        PreprocessResult {
            preprocessed_source: String::new(),
            stages: ShaderStageFlags::empty(),
            vertex_attributes: None,
            topology: None,
            source_map: Vec::new(),
            version: None,
            descriptor_map: Vec::new(),
            static_samplers: Vec::new(),
        }
    }
}

lazy_static! {
    static ref RE_INCLUDE: Regex = Regex::new(r#"^\s*#include\s+"(?P<path>.*)"\s*?$"#).unwrap();
    static ref RE_VERSION: Regex = Regex::new(r#"^\s*#version\s+(?P<version>[0-9]*)\s*?$"#).unwrap();

    static ref RE_PRAGMA: Regex = Regex::new(r#"^#pragma\s+(?P<directive>\w+)\s*(?P<params>.*)$"#).unwrap();
    static ref RE_PRAGMA_PRIMITIVE_TOPOLOGY: Regex = Regex::new(r#"^\((?P<topology>point|line|triangle)\)$"#).unwrap();
    static ref RE_PRAGMA_VERTEX_INPUT: Regex = Regex::new(r#"^\(location=(?P<location>\d+),b(?P<binding>\d+),(?P<format>\w+),offset=(?P<offset>\d+)\)$"#).unwrap();
    static ref RE_PRAGMA_DESCRIPTOR: Regex = Regex::new(r#"^\((?P<target_binding_range>(?:(?:t|u|i|s|a)\d+|(?:t\d+-t\d+|u\d+-u\d+|i\d+-i\d+|s\d+-s\d+|a\d+-a\d+))),set=(?P<set>\d+),binding=(?P<binding>\d+)\)$"#).unwrap();
    static ref RE_PRAGMA_SHADER_STAGE: Regex = Regex::new(r#"^\((?P<stages>(?:vertex|geometry|fragment|tess_control|tess_eval|compute)(?:,(?:vertex|geometry|fragment|tess_control|tess_eval|compute))*)\)$"#).unwrap();
    static ref RE_PRAGMA_STATIC_SAMPLER: Regex = Regex::new(r#"^\((?P<binding_range>t\d+-t\d+|t\d+),(?P<addr_u>wrap|clamp|mirror|border),(?P<addr_v>wrap|clamp|mirror|border),(?P<addr_w>wrap|clamp|mirror|border),(?P<min_filter>linear|nearest),(?P<mag_filter>linear|nearest),(?P<mipmap_mode>mip_linear|mip_nearest)\)$"#).unwrap();
}

#[derive(Debug)]
pub struct Error {
    pub source_index: u32,
    pub line: u32,
    pub kind: PreprocessErrorKind,
}

impl Error {
    pub fn new(source_index: u32, line: u32, kind: PreprocessErrorKind) -> Error {
        Error {
            source_index,
            line,
            kind,
        }
    }
}

#[derive(Debug)]
pub enum PreprocessErrorKind {
    UnableToOpenIncludeFile { path: PathBuf, error: io::Error },
    MalformedVersionDirective,
    MalformedPragmaDirective(Option<String>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "(source_index={})({}) ", self.source_index, self.line)?;
        match &self.kind {
            PreprocessErrorKind::UnableToOpenIncludeFile { path, error } => {
                write!(f, "Unable to open include file {:?}: {}", path, error)?;
            }
            PreprocessErrorKind::MalformedVersionDirective => {
                write!(f, "Malformed version directive")?;
            }
            PreprocessErrorKind::MalformedPragmaDirective(msg) => {
                if let Some(msg) = msg {
                    write!(f, "Malformed #pragma directive: {}", msg)?;
                } else {
                    write!(f, "Malformed #pragma directive")?;
                }
            }
            //_ => write!(f, "Unspecified error"),
        }
        Ok(())
    }
}

impl error::Error for Error {}

#[derive(Debug)]
pub struct PreprocessErrors(Vec<Error>);

impl fmt::Display for PreprocessErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unimplemented!()
    }
}

impl error::Error for PreprocessErrors {}

impl From<Vec<Error>> for PreprocessErrors {
    fn from(v: Vec<Error>) -> Self {
        PreprocessErrors(v)
    }
}

fn parse_binding_range(s: &str) -> (u32, u32) {
    if let Some(n) = s.find('-') {
        // got a range
        let (from, to) = s.split_at(n);
        let from = from.split_at(1).1.parse::<u32>().unwrap();
        let to = to.split_at(2).1.parse::<u32>().unwrap();
        (from, to)
    } else {
        // got a single binding number
        let v = s.split_at(1).1.parse::<u32>().unwrap();
        (v, v)
    }
}

fn parse_sampler_address_mode(s: &str) -> SamplerAddressMode {
    match s {
        "wrap" => SamplerAddressMode::Wrap,
        "clamp" => SamplerAddressMode::Clamp,
        "mirror" => SamplerAddressMode::Mirror,
        _ => unimplemented!("unimplemented sampler address mode: {}", s),
    }
}

fn parse_filter(s: &str) -> Filter {
    match s {
        "linear" => Filter::Linear,
        "nearest" => Filter::Nearest,
        _ => unimplemented!("unimplemented filter: {}", s),
    }
}

fn parse_mipmap_mode(s: &str) -> SamplerMipmapMode {
    match s {
        "mip_linear" => SamplerMipmapMode::Linear,
        "mip_nearest" => SamplerMipmapMode::Nearest,
        _ => unimplemented!("unimplemented mipmap filter: {}", s),
    }
}

/// Preprocesses a combined GLSL source file: extract the additional informations in the custom pragmas
/// and returns the result in (last_seen_version, enabled_pipeline_stages, input_layout, topology)
fn preprocess_shader_internal<'a>(
    source: &str,
    result: &mut PreprocessResult,
    include_paths: &[&'a Path],
    this_file: &IncludeFile<'a>,
    errors: &mut Vec<Error>,
) {
    let this_file_index = result.source_map.len() as u32;
    result.source_map.push(SourceMapEntry {
        index: this_file_index,
        path: this_file.path.as_ref().map(|p| p.to_path_buf()),
    });

    let mut should_output_line_directive = false;

    'line: for (cur_line, line) in source.lines().enumerate() {
        let cur_line = (cur_line + 1) as u32;

        if let Some(captures) = RE_INCLUDE.captures(line) {
            // INCLUDE -----------------------------------------------------------------------------

            let this_file_parent_directory = this_file.path.and_then(|p| p.parent());
            let filename = &captures["path"];

            // try all paths
            'include_paths: for &p in this_file_parent_directory.iter().chain(include_paths) {
                let mut inc_path = p.to_owned();
                inc_path.push(filename);
                debug!("include path = {:?}", &inc_path);

                match File::open(&inc_path) {
                    Ok(mut file) => {
                        let mut text = String::new();
                        file.read_to_string(&mut text).expect("failed to read file");
                        let next_include = IncludeFile {
                            path: Some(&inc_path),
                            parent: Some(&this_file),
                        };
                        preprocess_shader_internal(
                            &text,
                            result,
                            include_paths,
                            &next_include,
                            errors,
                        );

                        should_output_line_directive = true;
                        continue 'line;
                    }
                    Err(e) => {
                        match e.kind() {
                            io::ErrorKind::NotFound => {
                                // not found, continue and try another path
                                continue 'include_paths;
                            }
                            _ => {
                                // could not open include file
                                errors.push(Error::new(
                                    this_file_index,
                                    cur_line,
                                    PreprocessErrorKind::UnableToOpenIncludeFile {
                                        path: inc_path.clone(),
                                        error: e,
                                    },
                                ));
                            }
                        }
                        should_output_line_directive = true;
                        continue 'line;
                    }
                };
            }

            // not found
            errors.push(Error::new(
                this_file_index,
                cur_line,
                PreprocessErrorKind::UnableToOpenIncludeFile {
                    path: filename.into(),
                    error: io::Error::new(
                        io::ErrorKind::NotFound,
                        "include file was not found in any include path",
                    ),
                },
            ));

            should_output_line_directive = true;
            continue 'line;
        } else if let Some(captures) = RE_VERSION.captures(line) {
            // VERSION LINE ------------------------------------------------------------------------
            let version = if let Ok(version) = (&captures["version"]).parse::<u32>() {
                version
            } else {
                errors.push(Error::new(
                    this_file_index,
                    cur_line,
                    PreprocessErrorKind::MalformedVersionDirective,
                ));
                should_output_line_directive = true;
                continue 'line;
            };

            if let Some(previous_ver) = result.version {
                if previous_ver != version {
                    warn!(
                        "{:?}({:?}): version differs from previously specified version ({:?}, was {:?})",
                        this_file.path,
                        cur_line,
                        previous_ver,
                        version
                    );
                    result.version = Some(version);
                }
            } else {
                result.version = Some(version);
            };

            should_output_line_directive = true;
            continue 'line;
        } else if let Some(captures) = RE_PRAGMA.captures(line) {
            // PRAGMA DIRECTIVES -------------------------------------------------------------------
            debug!("pragma directive {}", line);
            let malformed_pragma_err = || {
                Error::new(
                    this_file_index,
                    cur_line,
                    PreprocessErrorKind::MalformedPragmaDirective(None),
                )
            };
            let directive = &captures["directive"];
            let params = &captures["params"];

            match directive {
                // #pragma stages ------------------------------------------------------------------
                "stages" => {
                    let captures = if let Some(captures) = RE_PRAGMA_SHADER_STAGE.captures(params) {
                        captures
                    } else {
                        errors.push(Error::new(this_file_index, cur_line, PreprocessErrorKind::MalformedPragmaDirective(Some(
                            "expected `vertex`, `fragment`, `tess_control`, `tess_eval`, `geometry` or `compute`".to_string()))));
                        should_output_line_directive = true;
                        continue 'line;
                    };

                    let stages = &captures["stages"];
                    for stage in stages.split(',').map(|s| s.trim()) {
                        match stage {
                            "vertex" => {
                                result.stages |= ShaderStageFlags::VERTEX;
                            }
                            "fragment" => {
                                result.stages |= ShaderStageFlags::FRAGMENT;
                            }
                            "geometry" => {
                                result.stages |= ShaderStageFlags::GEOMETRY;
                            }
                            "tess_control" => {
                                result.stages |= ShaderStageFlags::TESS_CONTROL;
                            }
                            "tess_eval" => {
                                result.stages |= ShaderStageFlags::TESS_EVAL;
                            }
                            "compute" => {
                                result.stages |= ShaderStageFlags::COMPUTE;
                            }
                            _ => {
                                errors.push(Error::new(this_file_index,
                                                       cur_line,
                                                       PreprocessErrorKind::MalformedPragmaDirective(
                                                                     Some(format!("unknown shader stage in `#pragma stage` directive: `{:?}`. \
                                        expected `vertex`, `fragment`, `tess_control`, `tess_eval`, `geometry` or `compute`", stage)))));
                            }
                        }
                    }

                    should_output_line_directive = true;
                    continue 'line;
                }

                // #pragma vertex_attribute --------------------------------------------------------
                "vertex_attribute" => {
                    let captures = if let Some(captures) = RE_PRAGMA_VERTEX_INPUT.captures(params) {
                        captures
                    } else {
                        errors.push(malformed_pragma_err());
                        continue 'line;
                    };

                    let location = (&captures["location"]).parse::<u32>().unwrap();
                    let binding = (&captures["binding"]).parse::<u32>().unwrap();
                    let offset = (&captures["offset"]).parse::<u32>().unwrap();

                    let format_str = &captures["format"];

                    let format = match format_str {
                        "rgba32f" => Format::R32G32B32A32_SFLOAT,
                        "rgb32f" => Format::R32G32B32_SFLOAT,
                        "rg32f" => Format::R32G32_SFLOAT,
                        "r32f" => Format::R32_SFLOAT,
                        "rgba16_snorm" => Format::R16G16B16A16_SNORM,
                        "rgb16_snorm" => Format::R16G16B16_SNORM,
                        "rg16_snorm" => Format::R16G16_SNORM,
                        "r16_snorm" => Format::R16_SNORM,
                        "rgba8_unorm" => Format::R8G8B8A8_UNORM,
                        "rgba8_snorm" => Format::R8G8B8A8_SNORM,
                        _ => {
                            errors.push(Error::new(this_file_index, cur_line, PreprocessErrorKind::MalformedPragmaDirective(Some(format!(
                                "unrecognized or unsupported format in `vertex_attribute` directive: {}",
                                format_str)))));
                            continue 'line;
                        }
                    };

                    let attrib = VertexInputAttributeDescription {
                        location,
                        binding,
                        format,
                        offset,
                    };

                    if let Some(ref mut vertex_attributes) = result.vertex_attributes {
                        vertex_attributes.push(attrib);
                    } else {
                        result.vertex_attributes = Some(vec![attrib]);
                    }
                }

                // #pragma descriptor --------------------------------------------------------------
                "descriptor" => {
                    let captures = if let Some(captures) = RE_PRAGMA_DESCRIPTOR.captures(params) {
                        captures
                    } else {
                        errors.push(malformed_pragma_err());
                        should_output_line_directive = true;
                        continue 'line;
                    };

                    let target_binding_range_str = &captures["target_binding_range"];
                    let target_binding_range = parse_binding_range(target_binding_range_str);

                    let target_binding_space =
                        match target_binding_range_str.chars().next().unwrap() {
                            't' => BindingSpace::Texture,
                            'i' => BindingSpace::Image,
                            's' => BindingSpace::ShaderStorageBuffer,
                            'u' => BindingSpace::UniformBuffer,
                            'a' => BindingSpace::AtomicCounterBuffer,
                            other => unimplemented!("unimplemented binding space '{}'", other),
                        };

                    let binding_base = (&captures["binding"]).parse::<u32>().unwrap();
                    let set = (&captures["set"]).parse::<u32>().unwrap();

                    let desc = DescriptorMapEntry {
                        set,
                        binding_base,
                        target_binding_space,
                        target_binding_range,
                    };

                    result.descriptor_map.push(desc);

                    should_output_line_directive = true;
                    continue 'line;
                }

                // #pragma sampler -----------------------------------------------------------------
                "sampler" => {
                    let captures = if let Some(captures) = RE_PRAGMA_STATIC_SAMPLER.captures(params)
                    {
                        captures
                    } else {
                        errors.push(malformed_pragma_err());
                        continue 'line;
                    };

                    let binding_range_str = &captures["binding_range"];
                    let texture_binding_range = parse_binding_range(binding_range_str);

                    let addr_u = parse_sampler_address_mode(&captures["addr_u"]);
                    let addr_v = parse_sampler_address_mode(&captures["addr_v"]);
                    let addr_w = parse_sampler_address_mode(&captures["addr_w"]);

                    let min_filter = parse_filter(&captures["min_filter"]);
                    let mag_filter = parse_filter(&captures["mag_filter"]);
                    let mipmap_mode = parse_mipmap_mode(&captures["mipmap_mode"]);

                    result.static_samplers.push(StaticSamplerEntry {
                        texture_binding_range,
                        description: SamplerDescription {
                            addr_u,
                            addr_v,
                            addr_w,
                            min_filter,
                            mag_filter,
                            mipmap_mode,
                        },
                    });

                    should_output_line_directive = true;
                    continue 'line;
                }
                // #pragma topology ----------------------------------------------------------------
                "topology" => {
                    let captures =
                        if let Some(captures) = RE_PRAGMA_PRIMITIVE_TOPOLOGY.captures(params) {
                            captures
                        } else {
                            errors.push(malformed_pragma_err());
                            should_output_line_directive = true;
                            continue 'line;
                        };

                    let topo_str = &captures["topology"];
                    let topology = match topo_str {
                        "point" => PrimitiveTopology::PointList,
                        "line" => PrimitiveTopology::LineList,
                        "triangle" => PrimitiveTopology::TriangleList,
                        _ => panic!("unsupported primitive topology: {}", topo_str),
                    };

                    if result.topology.is_some() {
                        warn!(
                            "{:?}({:?}) duplicate input_layout directive, ignoring",
                            this_file_index, cur_line
                        );
                    } else {
                        result.topology = Some(topology)
                    }

                    should_output_line_directive = true;
                    continue 'line;
                }
                // unrecognized pragma -------------------------------------------------------------
                _ => {
                    // do normal line processing
                }
            }
        }

        // NORMAL LINE PROCESSING --------------------------------------------------------------
        if should_output_line_directive {
            result
                .preprocessed_source
                .push_str(&format!("#line {} {}\n", cur_line, this_file_index));
            should_output_line_directive = false;
        }
        result.preprocessed_source.push_str(line);
        result.preprocessed_source.push('\n');
    }
}

// MAIN --------------------------------------------------------------------------------------------
pub fn preprocess_pipeline_description_file(
    source: &str,
    source_path: Option<&Path>,
    include_paths: &[&Path],
) -> Result<PreprocessResult, PreprocessErrors> {
    let mut result = PreprocessResult::new();
    let mut errors = Vec::new();

    let this_file = IncludeFile {
        path: source_path,
        parent: None,
    };

    preprocess_shader_internal(source, &mut result, include_paths, &this_file, &mut errors);

    if errors.is_empty() {
        Ok(result)
    } else {
        Err(errors.into())
    }
}

// -------------------------------------------------------------------------------------------------
pub struct SeparateShaderSources {
    pub vertex: Option<String>,
    pub fragment: Option<String>,
    pub tess_control: Option<String>,
    pub tess_eval: Option<String>,
    pub geometry: Option<String>,
    pub compute: Option<String>,
}

impl SeparateShaderSources {
    pub fn from_combined_source(
        src: &str,
        version: u32,
        stages: ShaderStageFlags,
        additional_macros: &[&str],
    ) -> SeparateShaderSources {
        lazy_static! {
            static ref RE_MACRO_DEF: Regex =
                Regex::new(r"^(?P<key>\w+)(?:=(?P<value>\w*))?$").unwrap();
        }

        let mut out_header = String::new();
        out_header.push_str(&format!("#version {}\n", version));

        for m in additional_macros {
            if let Some(captures) = RE_MACRO_DEF.captures(m) {
                out_header.push_str("#define ");
                out_header.push_str(&captures["key"]);
                if let Some(m) = captures.name("value") {
                    out_header.push_str(" ");
                    out_header.push_str(m.as_str());
                    out_header.push('\n');
                }
            } else {
                panic!("Malformed macro definition: {}", m);
            }
        }

        let gen_variant = |stage: ShaderStageFlags| {
            if stages.contains(stage) {
                let stage_def = match stage {
                    ShaderStageFlags::VERTEX => "_VERTEX_",
                    ShaderStageFlags::GEOMETRY => "_GEOMETRY_",
                    ShaderStageFlags::FRAGMENT => "_FRAGMENT_",
                    ShaderStageFlags::TESS_CONTROL => "_TESS_CONTROL_",
                    ShaderStageFlags::TESS_EVAL => "_TESS_EVAL_",
                    ShaderStageFlags::COMPUTE => "_COMPUTE_",
                    _ => panic!("invalid shader stage"),
                };
                let mut out = out_header.clone();
                out.push_str(&format!("#define {}\n", stage_def));
                out.push_str("#line 0 0\n");
                out.push_str(src);
                Some(out)
            } else {
                None
            }
        };

        SeparateShaderSources {
            vertex: gen_variant(ShaderStageFlags::VERTEX),
            geometry: gen_variant(ShaderStageFlags::GEOMETRY),
            fragment: gen_variant(ShaderStageFlags::FRAGMENT),
            tess_control: gen_variant(ShaderStageFlags::TESS_CONTROL),
            tess_eval: gen_variant(ShaderStageFlags::TESS_EVAL),
            compute: gen_variant(ShaderStageFlags::COMPUTE),
        }
    }
}

// TESTS -------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regexp_pragma() {
        RE_PRAGMA.captures("#version").is_none();
        RE_PRAGMA.captures("#pragma ").is_none();
        let c = RE_PRAGMA.captures("#pragma a").unwrap();
        assert_eq!(&c["directive"], "a");
        assert_eq!(&c["params"], "");
        let c = RE_PRAGMA.captures("#pragma     aaaa    ").unwrap();
        assert_eq!(&c["directive"], "aaaa");
        assert_eq!(&c["params"], "");
        let c = RE_PRAGMA.captures("#pragma 33    ").unwrap();
        assert_eq!(&c["directive"], "33");
        assert_eq!(&c["params"], "");
        let c = RE_PRAGMA.captures("#pragma directive()").unwrap();
        assert_eq!(&c["directive"], "directive");
        assert_eq!(&c["params"], "()");
        let c = RE_PRAGMA.captures("#pragma directive   ()").unwrap();
        assert_eq!(&c["directive"], "directive");
        assert_eq!(&c["params"], "()");
        let c = RE_PRAGMA.captures("#pragma directive(p1,p2,p3)").unwrap();
        assert_eq!(&c["directive"], "directive");
        assert_eq!(&c["params"], "(p1,p2,p3)");
    }

    #[test]
    fn test_regexp_stages() {
        assert!(RE_PRAGMA_SHADER_STAGE.captures("()").is_none());

        let c = RE_PRAGMA_SHADER_STAGE.captures("(vertex)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&c["stages"], "vertex");

        let c = RE_PRAGMA_SHADER_STAGE.captures("(fragment)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&c["stages"], "fragment");

        let c = RE_PRAGMA_SHADER_STAGE.captures("(tess_control)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&c["stages"], "tess_control");

        let c = RE_PRAGMA_SHADER_STAGE.captures("(tess_eval)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&c["stages"], "tess_eval");

        let c = RE_PRAGMA_SHADER_STAGE.captures("(geometry)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&c["stages"], "geometry");

        let c = RE_PRAGMA_SHADER_STAGE.captures("(compute)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(&c["stages"], "compute");

        let c = RE_PRAGMA_SHADER_STAGE
            .captures("(vertex,fragment,tess_control,tess_eval,geometry,compute)");
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(
            &c["stages"],
            "vertex,fragment,tess_control,tess_eval,geometry,compute"
        );
    }

    const SOURCE: &str = r#"
#version 440
#pragma stages(vertex,fragment)
#pragma topology(triangle)
#pragma vertex_attribute(location=0,b0,rgb32f,offset=0)
#pragma vertex_attribute(location=1,b0,rgb32f,offset=12)
#pragma vertex_attribute(location=2,b0,rgb32f,offset=24)
#pragma vertex_attribute(location=3,b0,rg32f,offset=36)
#pragma descriptor(u0-u7,set=0,binding=0)
#pragma descriptor(t0,set=0,binding=8)
#pragma sampler(t0-t7,wrap,clamp,mirror,linear,nearest,mip_linear)
"#;

    #[test]
    fn test_pp_shader_internal() {
        let mut result = GlslPreprocessResult {
            preprocessed_source: String::new(),
            stages: ShaderStageFlags::empty(),
            vertex_attributes: None,
            topology: None,
            source_map: Vec::new(),
            version: None,
            descriptor_map: Vec::new(),
            static_samplers: Vec::new(),
        };
        let this_file = IncludeFile {
            path: None,
            parent: None,
        };
        let mut errors = Vec::new();
        preprocess_shader_internal(SOURCE, &mut result, &[], &this_file, &mut errors);

        println!("errors: {:?}", errors);

        assert!(errors.is_empty());
        assert_eq!(
            result.stages,
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT
        );
        assert_eq!(
            result.vertex_attributes.as_ref().map(|a| a.as_ref()),
            Some(
                [
                    VertexInputAttribute {
                        format: Format::R32G32B32_SFLOAT,
                        location: 0,
                        binding: 0,
                        offset: 0
                    },
                    VertexInputAttribute {
                        format: Format::R32G32B32_SFLOAT,
                        location: 1,
                        binding: 0,
                        offset: 12
                    },
                    VertexInputAttribute {
                        format: Format::R32G32B32_SFLOAT,
                        location: 2,
                        binding: 0,
                        offset: 24
                    },
                    VertexInputAttribute {
                        format: Format::R32G32_SFLOAT,
                        location: 3,
                        binding: 0,
                        offset: 36
                    }
                ]
                .as_ref()
            )
        );
        assert_eq!(result.topology, Some(PrimitiveTopology::TriangleList));
        assert_eq!(result.source_map.len(), 1);
        assert_eq!(result.version, Some(440));
        assert_eq!(
            &result.descriptor_map,
            &[
                DescriptorMapEntry {
                    target_binding_range: (0, 7),
                    target_binding_space: BindingSpace::UniformBuffer,
                    set: 0,
                    binding_base: 0
                },
                DescriptorMapEntry {
                    target_binding_range: (0, 0),
                    target_binding_space: BindingSpace::Texture,
                    set: 0,
                    binding_base: 8
                }
            ]
        );
        assert_eq!(
            &result.static_samplers,
            &[StaticSamplerEntry {
                description: SamplerDescription {
                    addr_u: SamplerAddressMode::Wrap,
                    addr_v: SamplerAddressMode::Clamp,
                    addr_w: SamplerAddressMode::Mirror,
                    min_filter: Filter::Linear,
                    mag_filter: Filter::Nearest,
                    mipmap_mode: SamplerMipmapMode::Linear
                },
                texture_binding_range: (0, 7)
            }]
        );
    }
}

/*
pub fn preprocess_combined_shader_source<P: AsRef<Path>>(
    source: &str,
    path: P,
    macros: &[&str],
    _include_paths: &[&Path],
) -> (PipelineStages, PreprocessedShaders) {
    lazy_static! {
        static ref MACRO_DEF_RE: Regex = Regex::new(r"^(\w+)(?:=(\w*))?$").unwrap();
    }

    let this_file = IncludeFile {
        parent: None,
        path: path.as_ref(),
    };
    let mut source_map = Vec::new();
    let mut enabled_pipeline_stages = super::PipelineStages::empty();
    let mut glsl_version = None;
    let mut preprocessed = String::new();
    let mut input_layout = None;
    let mut primitive_topology = None;
    let num_errors = preprocess_shader_internal(
        &mut preprocessed,
        source,
        &mut glsl_version,
        &mut enabled_pipeline_stages,
        &mut input_layout,
        &mut primitive_topology,
        &this_file,
        &mut source_map,
    );
    debug!("PP: enabled stages: {:?}", enabled_pipeline_stages);
    debug!("PP: number of errors: {}", num_errors);

    let glsl_version = match glsl_version {
        Some(ver) => ver,
        None => {
            warn!("No #version directive found while preprocessing; defaulting to version 3.30");
            330
        }
    };

    debug!("PP: GLSL version = {}", glsl_version);
    debug!("PP: Source map:");
    for (i, f) in source_map.iter().enumerate() {
        debug!(" {} -> {:?} ", i, f.path);
    }

    let mut out_header = String::new();
    out_header.push_str(&format!("#version {}\n", glsl_version));
    for m in macros {
        if let Some(captures) = MACRO_DEF_RE.captures(m) {
            out_header.push_str("#define ");
            out_header.push_str(&captures[1]);
            if let Some(m) = captures.get(2) {
                out_header.push_str(" ");
                out_header.push_str(m.as_str());
                out_header.push('\n');
            }
        } else {
            // malformed macro
            panic!("Malformed macro definition: {}", m);
        }
    }

    let gen_variant = |stage: PipelineStages| {
        if enabled_pipeline_stages.contains(stage) {
            let stage_def = match stage {
                PS_VERTEX => "_VERTEX_",
                PS_GEOMETRY => "_GEOMETRY_",
                PS_FRAGMENT => "_FRAGMENT_",
                PS_TESS_CONTROL => "_TESS_CONTROL_",
                PS_TESS_EVAL => "_TESS_EVAL_",
                PS_COMPUTE => "_COMPUTE_",
                _ => panic!("Unexpected pattern"),
            };
            let mut out = out_header.clone();
            out.push_str(&format!("#define {}\n", stage_def));
            out.push_str("#line 0 0\n");
            out.push_str(&preprocessed);
            Some(out)
        } else {
            None
        }
    };

    (
        enabled_pipeline_stages,
        PreprocessedShaders {
            vertex: gen_variant(PS_VERTEX),
            geometry: gen_variant(PS_GEOMETRY),
            fragment: gen_variant(PS_FRAGMENT),
            tess_control: gen_variant(PS_TESS_CONTROL),
            tess_eval: gen_variant(PS_TESS_EVAL),
            compute: gen_variant(PS_COMPUTE),
            input_layout,
            primitive_topology,
        },
    )
}
*/
