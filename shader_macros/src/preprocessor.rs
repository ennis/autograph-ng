use lazy_static::lazy_static;
use regex::Regex;
use std::error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

//--------------------------------------------------------------------------------------------------
pub struct SourceMapEntry {
    pub index: u32,
    pub path: Option<PathBuf>,
}

pub struct PpIncludeResult {
    pub src: String,
    pub src_map: Vec<SourceMapEntry>,
    pub version: Option<u32>,
}

impl PpIncludeResult {
    fn new() -> PpIncludeResult {
        PpIncludeResult {
            src: String::new(),
            src_map: Vec::new(),
            version: None,
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "(src_index={})({}) ", self.src_index, self.line)?;
        match &self.kind {
            PreprocessErrorKind::UnableToOpenIncludeFile { path, err } => {
                write!(f, "unable to open include file {:?}: {}", path, err)?;
            }
            PreprocessErrorKind::MalformedVersionDirective => {
                write!(f, "malformed version directive")?;
            }
            /*PreprocessErrorKind::MalformedPragmaDirective(msg) => {
                if let Some(msg) = msg {
                    write!(f, "malformed #pragma directive: {}", msg)?;
                } else {
                    write!(f, "malformed #pragma directive")?;
                }
            }*/
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
        for err in self.0.iter() {
            fmt::Display::fmt(err, f)?;
        }
        Ok(())
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
    _parent: Option<&'a PpFile<'a>>,
    path: Option<&'a Path>,
}

lazy_static! {
    static ref RE_INCLUDE: Regex = Regex::new(r#"^\s*#include\s+"(?P<path>.*)"\s*?$"#).unwrap();
    static ref RE_VERSION: Regex =
        Regex::new(r#"^\s*#version\s+(?P<version>[0-9]*)\s*?$"#).unwrap();
    static ref RE_PRAGMA: Regex =
        Regex::new(r#"^#pragma\s+(?P<directive>\w+)\s*(?P<params>.*)$"#).unwrap();
}

//--------------------------------------------------------------------------------------------------
/// Preprocesses a combined GLSL source file: extract the additional informations in the custom pragmas
/// and returns the result in (last_seen_version, enabled_pipeline_stages, input_layout, topology)
fn process_includes_internal<'a>(
    src: &str,
    result: &mut PpIncludeResult,
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
                match File::open(&inc) {
                    Ok(mut incfile) => {
                        let mut text = String::new();
                        incfile
                            .read_to_string(&mut text)
                            .expect("failed to read file");
                        let nextinc = PpFile {
                            path: Some(&inc),
                            _parent: Some(&file),
                        };
                        process_includes_internal(&text, result, inc_paths, &nextinc, errs);

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
                        /*warn!(
                            "{:?}({:?}): version differs from previously specified version ({:?}, was {:?})",
                            file.path,
                            linei,
                            prev_ver,
                            ver
                        );*/
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
        }

        // NORMAL LINE PROCESSING --------------------------------------------------------------
        if should_output_line_directive {
            result.src.push_str(&format!("#line {} {}\n", linei, filei));
            should_output_line_directive = false;
        }
        result.src.push_str(line);
        result.src.push('\n');
    }
}

// MAIN --------------------------------------------------------------------------------------------
pub fn process_includes(
    src: &str,
    path: Option<&Path>,
    inc_paths: &[&Path],
) -> Result<PpIncludeResult, PreprocessErrors> {
    let mut result = PpIncludeResult::new();
    let mut errs = Vec::new();

    let ppfile = PpFile {
        path,
        _parent: None,
    };

    process_includes_internal(src, &mut result, inc_paths, &ppfile, &mut errs);

    if errs.is_empty() {
        Ok(result)
    } else {
        Err(errs.into())
    }
}

/// stage_id should be one of
/// * `"_VERTEX_"`
/// * `"_GEOMETRY_"`
/// * `"_FRAGMENT_"`
/// * `"_TESS_CONTROL_"`
/// * `"_TESS_EVAL_"`
/// * `"_COMPUTE_"`
pub fn extract_stage(src: &str, ver: u32, stage_macro: &str, macros: &[&str]) -> String {
    lazy_static! {
        static ref RE_MACRO_DEF: Regex = Regex::new(r"^(?P<key>\w+)(?:=(?P<value>\w*))?$").unwrap();
    }

    let mut out = String::new();
    out.push_str(&format!("#version {}\n", ver));

    for m in macros {
        if let Some(c) = RE_MACRO_DEF.captures(m) {
            out.push_str("#define ");
            out.push_str(&c["key"]);
            if let Some(m) = c.name("value") {
                out.push_str(" ");
                out.push_str(m.as_str());
                out.push('\n');
            }
        } else {
            panic!("malformed macro definition: {}", m);
        }
    }

    out.push_str(&format!("#define {}\n", stage_macro));
    out.push_str("#line 0 0\n");
    out.push_str(src);
    out
}

/*
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
}
*/
