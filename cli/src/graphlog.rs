// Copyright 2020 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::hash::Hash;
use std::io;
use std::io::Write;

use jj_lib::config::ConfigGetError;
use jj_lib::graph::GraphEdge;
use jj_lib::graph::GraphEdgeType;
use jj_lib::settings::UserSettings;
use renderdag::Ancestor;
use renderdag::GraphRowRenderer;
use renderdag::Renderer;

pub trait GraphLog<K: Clone + Eq + Hash> {
    fn add_node(
        &mut self,
        id: &K,
        edges: &[GraphEdge<K>],
        node_symbol: &str,
        text: &str,
    ) -> io::Result<()>;

    fn width(&self, id: &K, edges: &[GraphEdge<K>]) -> usize;
}

pub struct SaplingGraphLog<'writer, R> {
    renderer: R,
    writer: &'writer mut dyn Write,
    text_gap: usize,
}

const MESSAGE_MARKER: &str = "\u{1f}";

fn add_message_markers(text: &str) -> String {
    let mut lines = text.lines();
    let Some(first_line) = lines.next() else {
        return String::new();
    };

    let mut marked_text = format!("{MESSAGE_MARKER}{first_line}");
    for line in lines {
        marked_text.push('\n');
        marked_text.push_str(MESSAGE_MARKER);
        marked_text.push_str(line);
    }
    marked_text
}

fn remove_message_markers(row: &str, text_gap: usize) -> String {
    let mut output = String::with_capacity(row.len());
    for line in row.split_inclusive('\n') {
        let (line, line_ending) = line
            .strip_suffix('\n')
            .map_or((line, ""), |line| (line, "\n"));
        if let Some(marker_index) = line.find(MESSAGE_MARKER) {
            let prefix = &line[..marker_index];
            let suffix = &line[marker_index + MESSAGE_MARKER.len()..];
            if suffix.is_empty() {
                output.push_str(prefix.trim_end());
            } else if text_gap == 0 {
                output.push_str(prefix.strip_suffix(' ').unwrap_or(prefix));
            } else {
                output.push_str(prefix);
                output.extend(std::iter::repeat_n(' ', text_gap - 1));
            }
            output.push_str(suffix);
        } else {
            output.push_str(line);
        }
        output.push_str(line_ending);
    }
    output
}

fn convert_graph_edge_into_ancestor<K: Clone>(e: &GraphEdge<K>) -> Ancestor<K> {
    match e.edge_type {
        GraphEdgeType::Direct => Ancestor::Parent(e.target.clone()),
        GraphEdgeType::Indirect => Ancestor::Ancestor(e.target.clone()),
        GraphEdgeType::Missing => Ancestor::Anonymous,
    }
}

impl<K, R> GraphLog<K> for SaplingGraphLog<'_, R>
where
    K: Clone + Eq + Hash,
    R: Renderer<K, Output = String>,
{
    fn add_node(
        &mut self,
        id: &K,
        edges: &[GraphEdge<K>],
        node_symbol: &str,
        text: &str,
    ) -> io::Result<()> {
        let text = add_message_markers(text);
        let row = self.renderer.next_row(
            id.clone(),
            edges.iter().map(convert_graph_edge_into_ancestor).collect(),
            node_symbol.into(),
            text,
        );
        let row = remove_message_markers(&row, self.text_gap);

        write!(self.writer, "{row}")
    }

    fn width(&self, id: &K, edges: &[GraphEdge<K>]) -> usize {
        let parents = edges.iter().map(convert_graph_edge_into_ancestor).collect();
        let w: u64 = self.renderer.width(Some(id), Some(&parents));
        let width = usize::try_from(w).unwrap();
        // The renderer's default width includes one space separating the graph
        // from the rendered text.
        width.saturating_sub(1) + self.text_gap
    }
}

impl<'writer, R> SaplingGraphLog<'writer, R> {
    pub fn create<K>(
        renderer: R,
        formatter: &'writer mut dyn Write,
        text_gap: usize,
    ) -> Box<dyn GraphLog<K> + 'writer>
    where
        K: Clone + Eq + Hash + 'writer,
        R: Renderer<K, Output = String> + 'writer,
    {
        Box::new(SaplingGraphLog {
            renderer,
            writer: formatter,
            text_gap,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum GraphStyle {
    Ascii,
    AsciiLarge,
    Curved,
    Square,
}

impl GraphStyle {
    pub fn from_settings(settings: &UserSettings) -> Result<Self, ConfigGetError> {
        settings.get("ui.graph.style")
    }
}

pub fn text_gap_from_settings(settings: &UserSettings) -> Result<usize, ConfigGetError> {
    settings.get("ui.graph.text-gap")
}
pub fn get_graphlog<'a, K: Clone + Eq + Hash + 'a>(
    style: GraphStyle,
    text_gap: usize,
    formatter: &'a mut dyn Write,
) -> Box<dyn GraphLog<K> + 'a> {
    let builder = GraphRowRenderer::new().output().with_min_row_height(0);
    match style {
        GraphStyle::Ascii => SaplingGraphLog::create(builder.build_ascii(), formatter, text_gap),
        GraphStyle::AsciiLarge => {
            SaplingGraphLog::create(builder.build_ascii_large(), formatter, text_gap)
        }
        GraphStyle::Curved => {
            SaplingGraphLog::create(builder.build_box_drawing(), formatter, text_gap)
        }
        GraphStyle::Square => SaplingGraphLog::create(
            builder.build_box_drawing().with_square_glyphs(),
            formatter,
            text_gap,
        ),
    }
}
