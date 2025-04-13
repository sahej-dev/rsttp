use core::fmt;
use std::{collections::HashMap, error::Error, str::FromStr};

use tracing::{info, instrument};

#[derive(Debug)]
pub struct Path {
    parts: Vec<PathPart>,
}

impl Path {
    #[instrument]
    pub fn parse(path: &str) -> Result<Path, PathParseError> {
        if !path.starts_with("/") || !path.contains("/") {
            return Err(PathParseError {});
        }

        let path_parts: Vec<&str> = path.split('/').filter(|e| !e.is_empty()).collect();
        let path_parts: Result<Vec<PathPart>, PathPartParseError> = path_parts
            .iter()
            .map(|part| PathPart::from_str(part))
            .collect();

        if let Ok(parts) = path_parts {
            info!(?parts, "Generated Parts");
            return Ok(Path { parts });
        }

        info!("Path Parse Error");

        Err(PathParseError {})
    }

    pub fn get_req_param(&self, req_path: &Path) -> Option<HashMap<String, String>> {
        let matched_parts: Option<Vec<(PathPart, PathPart)>> = self.get_if_matches(req_path);

        matched_parts.map(|parts| {
            parts
                .iter()
                .filter_map(|(a, b)| {
                    if a.part_type == PathPartType::Dynamic && b.part_type == PathPartType::Static {
                        Some((a.part.clone(), b.part.clone()))
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    fn get_if_matches(&self, other: &Path) -> Option<Vec<(PathPart, PathPart)>> {
        if self != other {
            return None;
        }

        Some(
            self.parts
                .iter()
                .zip(&other.parts)
                .map(|(a, b)| (a.clone(), b.clone()))
                .collect(),
        )
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.parts.len() == other.parts.len()
            && self
                .parts
                .iter()
                .zip(&other.parts)
                .all(|(a, b)| a.part_type != PathPartType::Static || a.part == b.part)
    }
}

#[derive(Debug)]
pub struct PathParseError {}

impl fmt::Display for PathParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Failed to Parse str to a Path")
    }
}

impl Error for PathParseError {}

#[derive(Debug, Clone)]
pub struct PathPart {
    part: String,
    part_type: PathPartType,
}

#[derive(Debug)]
pub struct PathPartParseError {}

impl Error for PathPartParseError {}

impl fmt::Display for PathPartParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to Parse PathPart")
    }
}

impl FromStr for PathPart {
    type Err = PathPartParseError;

    #[instrument]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            info!("Empty part path");
            return Err(PathPartParseError {});
        }

        if let Some(stripped) = s.strip_prefix(":") {
            Ok(Self {
                part: stripped.to_string(),
                part_type: PathPartType::Dynamic,
            })
        } else {
            if let Some(c) = s.chars().next() {
                if !c.is_alphabetic() {
                    return Err(PathPartParseError {});
                }
            }

            Ok(Self {
                part: s.to_string(),
                part_type: PathPartType::Static,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PathPartType {
    Static,
    Dynamic,
}
