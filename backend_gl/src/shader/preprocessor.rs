use lazy_static::lazy_static;
use regex::Regex;
use std::error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::pipeline::{BindingSpace, StaticSamplerEntry};
use gfx2::{
    Filter, Format, PrimitiveTopology, SamplerAddressMode, SamplerDescription, SamplerMipmapMode,
    ShaderStageFlags, VertexInputAttributeDescription,
};

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
pub struct ParsedDescriptorMapping {
    pub gl_space: BindingSpace,
    pub gl_range: (u32, u32),
    pub set: u32,
    pub binding_base: u32,
}

pub struct SourceMapEntry {
    pub index: u32,
    pub path: Option<PathBuf>,
}

pub struct PreprocessResult {
    pub srcpp: String,
    pub stages: ShaderStageFlags,
    pub attribs: Option<Vec<VertexInputAttributeDescription>>,
    pub topo: Option<PrimitiveTopology>,
    pub src_map: Vec<SourceMapEntry>,
    pub version: Option<u32>,
    pub desc_map: Vec<ParsedDescriptorMapping>,
    pub samplers: Vec<StaticSamplerEntry>,
}

impl PreprocessResult {
    fn new() -> PreprocessResult {
        PreprocessResult {
            srcpp: String::new(),
            stages: ShaderStageFlags::empty(),
            attribs: None,
            topo: None,
            src_map: Vec::new(),
            version: None,
            desc_map: Vec::new(),
            samplers: Vec::new(),
        }
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct Error {
    pub src_index: u32,
    pub line: u32,
    pub kind: PreprocessErrorKind,
}

impl Error {
    pub fn new(src_index: u32, line: u32, kind: PreprocessErrorKind) -> Error {
        Error {
            src_index,
            line,
            kind,
        }
    }
}

#[derive(Debug)]
pub enum PreprocessErrorKind {
    UnableToOpenIncludeFile { path: PathBuf, err: io::Error },
    MalformedVersionDirective,
    MalformedPragmaDirective(Option<String>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "(src_index={})({}) ", self.src_index, self.line)?;
        match &self.kind {
            PreprocessErrorKind::UnableToOpenIncludeFile { path, err } => {
                write!(f, "Unable to open include file {:?}: {}", path, err)?;
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
    fn fmt(&self, _f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unimplemented!()
    }
}

impl error::Error for PreprocessErrors {}

impl From<Vec<Error>> for PreprocessErrors {
    fn from(v: Vec<Error>) -> Self {
        PreprocessErrors(v)
    }
}

//--------------------------------------------------------------------------------------------------

struct PpFile<'a> {
    parent: Option<&'a PpFile<'a>>,
    path: Option<&'a Path>,
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

//--------------------------------------------------------------------------------------------------

/// Preprocesses a combined GLSL source file: extract the additional informations in the custom pragmas
/// and returns the result in (last_seen_version, enabled_pipeline_stages, input_layout, topology)
fn preprocess_shader_internal<'a>(
    src: &str,
    result: &mut PreprocessResult,
    inc_paths: &[&'a Path],
    file: &PpFile<'a>,
    errs: &mut Vec<Error>,
) {
    let filei = result.src_map.len() as u32;
    result.src_map.push(SourceMapEntry {
        index: filei,
        path: file.path.as_ref().map(|p| p.to_path_buf()),
    });

    let mut should_output_line_directive = false;

    'line: for (linei, line) in src.lines().enumerate() {
        let linei = (linei + 1) as u32;

        if let Some(captures) = RE_INCLUDE.captures(line) {
            // INCLUDE -----------------------------------------------------------------------------

            let parent_dir = file.path.and_then(|p| p.parent());
            let filename = &captures["path"];

            // try all paths
            'include_paths: for &p in parent_dir.iter().chain(inc_paths) {
                let mut inc = p.to_owned();
                inc.push(filename);
                //debug!("include path = {:?}", &inc);

                match File::open(&inc) {
                    Ok(mut incfile) => {
                        let mut text = String::new();
                        incfile
                            .read_to_string(&mut text)
                            .expect("failed to read file");
                        let nextinc = PpFile {
                            path: Some(&inc),
                            parent: Some(&file),
                        };
                        preprocess_shader_internal(&text, result, inc_paths, &nextinc, errs);

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
                                errs.push(Error::new(
                                    filei,
                                    linei,
                                    PreprocessErrorKind::UnableToOpenIncludeFile {
                                        path: inc.clone(),
                                        err: e,
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
            errs.push(Error::new(
                filei,
                linei,
                PreprocessErrorKind::UnableToOpenIncludeFile {
                    path: filename.into(),
                    err: io::Error::new(
                        io::ErrorKind::NotFound,
                        "include file was not found in any include path",
                    ),
                },
            ));

            should_output_line_directive = true;
            continue 'line;
        } else if let Some(cap) = RE_VERSION.captures(line) {
            // VERSION LINE ------------------------------------------------------------------------
            if let Ok(ver) = (&cap["version"]).parse::<u32>() {
                if let Some(prev_ver) = result.version {
                    if prev_ver != ver {
                        warn!(
                            "{:?}({:?}): version differs from previously specified version ({:?}, was {:?})",
                            file.path,
                            linei,
                            prev_ver,
                            ver
                        );
                        result.version = Some(ver);
                    }
                } else {
                    result.version = Some(ver);
                }
            } else {
                errs.push(Error::new(
                    filei,
                    linei,
                    PreprocessErrorKind::MalformedVersionDirective,
                ));
            }

            should_output_line_directive = true;
            continue 'line;
        } else if let Some(c) = RE_PRAGMA.captures(line) {
            // PRAGMA DIRECTIVES -------------------------------------------------------------------
            //debug!("pragma directive {}", line);
            let malformed_pragma_err = || {
                Error::new(
                    filei,
                    linei,
                    PreprocessErrorKind::MalformedPragmaDirective(None),
                )
            };
            let directive = &c["directive"];
            let params = &c["params"];

            match directive {
                // #pragma stages ------------------------------------------------------------------
                "stages" => {
                    if let Some(c) = RE_PRAGMA_SHADER_STAGE.captures(params) {
                        let stages = &c["stages"];
                        for stg in stages.split(',').map(|s| s.trim()) {
                            match stg {
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
                                    errs.push(Error::new(filei,
                                                         linei,
                                                         PreprocessErrorKind::MalformedPragmaDirective(
                                                             Some(format!("unknown shader stage in `#pragma stage` directive: `{:?}`. \
                                        expected `vertex`, `fragment`, `tess_control`, `tess_eval`, `geometry` or `compute`", stg)))));
                                }
                            }
                        }
                    } else {
                        errs.push(Error::new(filei, linei, PreprocessErrorKind::MalformedPragmaDirective(Some(
                            "expected `vertex`, `fragment`, `tess_control`, `tess_eval`, `geometry` or `compute`".to_string()))));
                    }

                    should_output_line_directive = true;
                    continue 'line;
                }

                // #pragma vertex_attribute --------------------------------------------------------
                "vertex_attribute" => {
                    if let Some(c) = RE_PRAGMA_VERTEX_INPUT.captures(params) {
                        let location = (&c["location"]).parse::<u32>().unwrap();
                        let binding = (&c["binding"]).parse::<u32>().unwrap();
                        let offset = (&c["offset"]).parse::<u32>().unwrap();

                        let format_str = &c["format"];

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
                                errs.push(Error::new(filei, linei, PreprocessErrorKind::MalformedPragmaDirective(Some(format!(
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

                        if let Some(ref mut attribs) = result.attribs {
                            attribs.push(attrib);
                        } else {
                            result.attribs = Some(vec![attrib]);
                        }
                    } else {
                        errs.push(malformed_pragma_err());
                    }

                    should_output_line_directive = true;
                    continue 'line;
                }

                // #pragma descriptor --------------------------------------------------------------
                "descriptor" => {
                    if let Some(c) = RE_PRAGMA_DESCRIPTOR.captures(params) {
                        let gl_range_str = &c["target_binding_range"];
                        let gl_range = parse_binding_range(gl_range_str);

                        let gl_space = match gl_range_str.chars().next().unwrap() {
                            't' => BindingSpace::Texture,
                            'i' => BindingSpace::Image,
                            's' => BindingSpace::ShaderStorageBuffer,
                            'u' => BindingSpace::UniformBuffer,
                            'a' => BindingSpace::AtomicCounterBuffer,
                            other => unimplemented!("unimplemented binding space '{}'", other),
                        };

                        let binding_base = (&c["binding"]).parse::<u32>().unwrap();
                        let set = (&c["set"]).parse::<u32>().unwrap();

                        let desc = ParsedDescriptorMapping {
                            set,
                            binding_base,
                            gl_space,
                            gl_range,
                        };

                        result.desc_map.push(desc);
                    } else {
                        errs.push(malformed_pragma_err());
                    }

                    should_output_line_directive = true;
                    continue 'line;
                }

                // #pragma sampler -----------------------------------------------------------------
                "sampler" => {
                    if let Some(c) = RE_PRAGMA_STATIC_SAMPLER.captures(params) {
                        let tex_range_str = &c["binding_range"];
                        let tex_range = parse_binding_range(tex_range_str);

                        let addr_u = parse_sampler_address_mode(&c["addr_u"]);
                        let addr_v = parse_sampler_address_mode(&c["addr_v"]);
                        let addr_w = parse_sampler_address_mode(&c["addr_w"]);

                        let min_filter = parse_filter(&c["min_filter"]);
                        let mag_filter = parse_filter(&c["mag_filter"]);
                        let mipmap_mode = parse_mipmap_mode(&c["mipmap_mode"]);

                        result.samplers.push(StaticSamplerEntry {
                            tex_range,
                            desc: SamplerDescription {
                                addr_u,
                                addr_v,
                                addr_w,
                                min_filter,
                                mag_filter,
                                mipmap_mode,
                            },
                        })
                    } else {
                        errs.push(malformed_pragma_err());
                    }

                    should_output_line_directive = true;
                    continue 'line;
                }
                // #pragma topology ----------------------------------------------------------------
                "topology" => {
                    if let Some(c) = RE_PRAGMA_PRIMITIVE_TOPOLOGY.captures(params) {
                        let topo_str = &c["topology"];
                        let topology = match topo_str {
                            "point" => PrimitiveTopology::PointList,
                            "line" => PrimitiveTopology::LineList,
                            "triangle" => PrimitiveTopology::TriangleList,
                            _ => panic!("unsupported primitive topology: {}", topo_str),
                        };

                        if result.topo.is_some() {
                            warn!(
                                "{:?}({:?}) duplicate input_layout directive, ignoring",
                                filei, linei
                            );
                        } else {
                            result.topo = Some(topology)
                        }
                    } else {
                        errs.push(malformed_pragma_err());
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
                .srcpp
                .push_str(&format!("#line {} {}\n", linei, filei));
            should_output_line_directive = false;
        }
        result.srcpp.push_str(line);
        result.srcpp.push('\n');
    }
}

// MAIN --------------------------------------------------------------------------------------------
pub fn preprocess_pipeline_description_file(
    src: &str,
    path: Option<&Path>,
    inc_paths: &[&Path],
) -> Result<PreprocessResult, PreprocessErrors> {
    let mut result = PreprocessResult::new();
    let mut errs = Vec::new();

    let ppfile = PpFile { path, parent: None };

    preprocess_shader_internal(src, &mut result, inc_paths, &ppfile, &mut errs);

    if errs.is_empty() {
        Ok(result)
    } else {
        Err(errs.into())
    }
}

// -------------------------------------------------------------------------------------------------
pub struct SeparateShaderSources {
    pub vert: Option<String>,
    pub frag: Option<String>,
    pub tessctl: Option<String>,
    pub tesseval: Option<String>,
    pub geom: Option<String>,
    pub comp: Option<String>,
}

impl SeparateShaderSources {
    pub fn from_combined_source(
        src: &str,
        ver: u32,
        stages: ShaderStageFlags,
        macros: &[&str],
    ) -> SeparateShaderSources {
        lazy_static! {
            static ref RE_MACRO_DEF: Regex =
                Regex::new(r"^(?P<key>\w+)(?:=(?P<value>\w*))?$").unwrap();
        }

        let mut hdr = String::new();
        hdr.push_str(&format!("#version {}\n", ver));

        for m in macros {
            if let Some(c) = RE_MACRO_DEF.captures(m) {
                hdr.push_str("#define ");
                hdr.push_str(&c["key"]);
                if let Some(m) = c.name("value") {
                    hdr.push_str(" ");
                    hdr.push_str(m.as_str());
                    hdr.push('\n');
                }
            } else {
                panic!("Malformed macro definition: {}", m);
            }
        }

        let variant = |stage: ShaderStageFlags| {
            if stages.contains(stage) {
                let stagedef = match stage {
                    ShaderStageFlags::VERTEX => "_VERTEX_",
                    ShaderStageFlags::GEOMETRY => "_GEOMETRY_",
                    ShaderStageFlags::FRAGMENT => "_FRAGMENT_",
                    ShaderStageFlags::TESS_CONTROL => "_TESS_CONTROL_",
                    ShaderStageFlags::TESS_EVAL => "_TESS_EVAL_",
                    ShaderStageFlags::COMPUTE => "_COMPUTE_",
                    _ => panic!("invalid shader stage"),
                };
                let mut out = hdr.clone();
                out.push_str(&format!("#define {}\n", stagedef));
                out.push_str("#line 0 0\n");
                out.push_str(src);
                Some(out)
            } else {
                None
            }
        };

        SeparateShaderSources {
            vert: variant(ShaderStageFlags::VERTEX),
            geom: variant(ShaderStageFlags::GEOMETRY),
            frag: variant(ShaderStageFlags::FRAGMENT),
            tessctl: variant(ShaderStageFlags::TESS_CONTROL),
            tesseval: variant(ShaderStageFlags::TESS_EVAL),
            comp: variant(ShaderStageFlags::COMPUTE),
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
        let mut result = PreprocessResult {
            srcpp: String::new(),
            stages: ShaderStageFlags::empty(),
            attribs: None,
            topo: None,
            src_map: Vec::new(),
            version: None,
            desc_map: Vec::new(),
            samplers: Vec::new(),
        };
        let this_file = PpFile {
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
            result.attribs.as_ref().map(|a| a.as_ref()),
            Some(
                [
                    VertexInputAttributeDescription {
                        format: Format::R32G32B32_SFLOAT,
                        location: 0,
                        binding: 0,
                        offset: 0
                    },
                    VertexInputAttributeDescription {
                        format: Format::R32G32B32_SFLOAT,
                        location: 1,
                        binding: 0,
                        offset: 12
                    },
                    VertexInputAttributeDescription {
                        format: Format::R32G32B32_SFLOAT,
                        location: 2,
                        binding: 0,
                        offset: 24
                    },
                    VertexInputAttributeDescription {
                        format: Format::R32G32_SFLOAT,
                        location: 3,
                        binding: 0,
                        offset: 36
                    }
                ]
                .as_ref()
            )
        );
        assert_eq!(result.topo, Some(PrimitiveTopology::TriangleList));
        assert_eq!(result.src_map.len(), 1);
        assert_eq!(result.version, Some(440));
        assert_eq!(
            &result.desc_map,
            &[
                ParsedDescriptorMapping {
                    gl_range: (0, 7),
                    gl_space: BindingSpace::UniformBuffer,
                    set: 0,
                    binding_base: 0
                },
                ParsedDescriptorMapping {
                    gl_range: (0, 0),
                    gl_space: BindingSpace::Texture,
                    set: 0,
                    binding_base: 8
                }
            ]
        );
        assert_eq!(
            &result.samplers,
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
