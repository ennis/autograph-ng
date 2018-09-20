//! Import a graph from a TOML file
use std::path::Path;
use std::fs;
use std::fs::File;
use std::collections::HashMap;

use frame::Frame;

use toml;
use serde::Deserialize;
use ash::vk;

#[derive(Debug, Deserialize)]
struct Image
{
    width: toml::Value,
    height: toml::Value,
    format: String
}

#[derive(Debug, Deserialize)]
struct ColorAttachment
{
    index: u32,
    id: String,
}

#[derive(Debug, Deserialize)]
struct Task
{
    #[serde(default)]
    color_attachments: Vec<ColorAttachment>,
    #[serde(default)]
    depth_attachment: Option<String>,
    #[serde(default)]
    image_sample: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PresentTask
{
    id: String,
}

#[derive(Debug, Deserialize)]
struct Graph
{
    #[serde(default)]
    vars: HashMap<String, toml::Value>,
    #[serde(default)]
    images: HashMap<String, Image>,
    #[serde(default)]
    tasks: HashMap<String, Task>,
    #[serde(default)]
    present: Vec<PresentTask>
}

fn parse_var<'de, T: Deserialize<'de>>(var: &'de toml::Value, vars: &HashMap<String, toml::Value>) -> T
{
    match var {
        toml::Value::String(s) => {
            if let Some('$') = s.chars().next() {
                let (_, rest) = s.split_at(1);
                <T as Deserialize>::deserialize(vars.get(rest).unwrap().clone()).unwrap()
            } else {
                <T as Deserialize>::deserialize(var.clone()).unwrap()
            }
        },
        _ => <T as Deserialize>::deserialize(var.clone()).unwrap()
    }
}

pub fn import_graph(path: impl AsRef<Path>, frame: &mut Frame)
{
    // open toml file
    let cfg_str = fs::read_to_string(path).unwrap();
    let g = toml::from_str::<Graph>(&cfg_str).unwrap();

    //let mut task_ids = HashMap::new();
    let mut images = HashMap::new();

    // images
    for (k,v) in g.images.iter() {
        let width = parse_var::<u32>(&v.width, &g.vars);
        let height = parse_var::<u32>(&v.width, &g.vars);
        let img = frame.create_image_2d((width, height), vk::Format::R16g16b16a16Sfloat);
        debug!("create image 2d {}x{}", width, height);
        images.insert(format!("{}@init", k), img);
    }

    // tasks
    for (k,v) in g.tasks.iter() {
        let t = frame.create_task(k.clone());
        // color attachments
        for color_attachment in v.color_attachments.iter() {
            let out = {
                let a = images.get(&color_attachment.id).expect(&format!("dependency not found: {}", color_attachment.id));
                frame.color_attachment_dependency(t, color_attachment.index, a)
                // drop borrow of images
            };
            let res_name = color_attachment.id.split('@').next().unwrap();
            images.insert(format!("{}@{}", res_name, k), out);
        }
        // image samples
        for sample in v.image_sample.iter() {
            let out = {
                let a = images.get(sample).expect("dependency not found");
                frame.image_sample_dependency(t, a)
                // drop borrow of images
            };
            //let res_name = color_attachment.id.split('@').next().unwrap();
            //images.insert(format!("{}@{}", res_name, k), out);
        }
    }

    // present tasks
    for present in g.present.iter() {
        let a = images.get(&present.id).expect("dependency not found");
        frame.present(a);
    }

}