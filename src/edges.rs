use image::DynamicImage;

use crate::error;
use glam::Vec2;

pub struct Edges {
    i: DynamicImage,
}

impl Edges {
    /// If there's only one sprite / object in the image, this returns just one, with
    /// coordinates translated to either side of (0, 0)
    pub fn single_image_edge_translated(&self) -> Vec<Vec2> {
        self.image_to_edges(true).into_iter().flatten().collect()
    }

    /// If there's only one sprite / object in the image, this returns just one, with
    /// coordinates left alone and all in positive x and y
    pub fn single_image_edge_raw(&self) -> Vec<Vec2> {
        self.image_to_edges(false).into_iter().flatten().collect()
    }

    /// If there's more than one sprite / object in the image, this returns all it finds, with
    /// coordinates translated to either side of (0, 0)
    pub fn multi_image_edge_translated(&self) -> Vec<Vec<Vec2>> {
        self.image_to_edges(true)
    }

    /// If there's more than one sprite / object in the image, this returns all it finds, with
    /// coordinates left alone and all in positive x and y
    pub fn multi_image_edges_raw(&self) -> Vec<Vec<Vec2>> {
        self.image_to_edges(false)
    }

    /// Takes a Bevy DynamicImage type and an boolean to indicate whether to translate
    /// the points you get back to either side of (0, 0) instead of everything in positive x and y
    pub fn image_to_edges(&self, translate: bool) -> Vec<Vec<Vec2>> {
        let rows = (self.i.height()) as usize;
        let cols = (self.i.width()) as usize;
        let data: &[u8] = self.i.as_bytes();
        let mut byte_combine_step: usize = 1;
        if (rows * cols) < data.len() {
            byte_combine_step = data.len() / (rows * cols);
        }

        let mut processed: Vec<usize> = vec![];
        for i in (0..data.len()).step_by(byte_combine_step) {
            let mut b: usize = 0;
            for j in 0..byte_combine_step {
                b |= data[i + j] as usize; // just need to retain any non-zero values
            }
            processed.push(b);
        }

        self.march_edges(&processed, rows, cols, translate)
    }

    /// Marching squares adjacent, walks all the pixels in the provided data and keeps track of
    /// any that have at least one transparent / zero value neighbor then, while sorting into drawing
    /// order, groups them into sets of connected pixels
    ///
    /// Accepts a flag indicating whether or not to translate coordinates to either side of (0,0)
    /// or leave it all in positive x,y
    pub fn march_edges(
        &self,
        data: &[usize],
        rows: usize,
        cols: usize,
        translate: bool,
    ) -> Vec<Vec<Vec2>> {
        let mut edge_points: Vec<Vec2> = vec![];

        for d in 0..data.len() {
            let (x, y) = self.get_xy(d, rows);
            let (c, r) = (x as isize, y as isize);

            if self.get_at(r, c, rows, cols, data) == 0 {
                continue;
            }

            let neighbors = [
                self.get_at(r + 1, c, rows, cols, data),
                self.get_at(r - 1, c, rows, cols, data),
                self.get_at(r, c + 1, rows, cols, data),
                self.get_at(r, c - 1, rows, cols, data),
                self.get_at(r + 1, c + 1, rows, cols, data),
                self.get_at(r - 1, c - 1, rows, cols, data),
                self.get_at(r + 1, c - 1, rows, cols, data),
                self.get_at(r - 1, c + 1, rows, cols, data),
            ];

            let n: usize = neighbors.iter().sum();
            let surrounded = neighbors.len();
            if n < surrounded {
                edge_points.push(Vec2::new(x, y));
            }
        }

        self.points_to_drawing_order(&edge_points, translate, rows, cols)
    }

    /// Takes a collection of coordinates and attempts to sort them according to drawing order
    ///
    /// Pixel sorted so that the distance to previous and next is 1. When there is no pixel left
    /// with distance 1, another group is created and sorted the same way.
    fn points_to_drawing_order(
        &self,
        points: &[Vec2],
        translate: bool,
        rows: usize,
        cols: usize,
    ) -> Vec<Vec<Vec2>> {
        let mut edge_points: Vec<Vec2> = points.to_vec();
        let mut in_drawing_order: Vec<Vec2> = vec![];
        let mut groups: Vec<Vec<Vec2>> = vec![];
        while !edge_points.is_empty() {
            if in_drawing_order.is_empty() {
                in_drawing_order.push(edge_points.swap_remove(0));
            }

            let prev = *in_drawing_order.last().unwrap();

            let neighbor = edge_points
                .iter()
                .enumerate()
                .find(|(_, p)| self.distance(prev, **p) == 1.0);

            if let Some((i, _)) = neighbor {
                let next = edge_points.remove(i);
                in_drawing_order.push(next);
                continue;
            }

            if !in_drawing_order.is_empty() {
                groups.push(in_drawing_order.clone());
                in_drawing_order.clear()
            }
        }

        if !in_drawing_order.is_empty() {
            groups.push(in_drawing_order.clone());
        }

        if translate {
            groups = groups
                .into_iter()
                .map(|p| self.translate_vec(p, rows, cols))
                .collect();
        }

        groups
    }

    /// conceptual helper, access a 1D vector like it's a 2D vector
    fn get_xy(&self, idx: usize, offset: usize) -> (f32, f32) {
        let quot = idx / offset;
        let rem = idx % offset;
        (quot as f32, rem as f32)
    }

    /// pythagoras, distance between two points
    fn distance(&self, a: Vec2, b: Vec2) -> f32 {
        // d=√((x2-x1)²+(y2-y1)²)
        ((a.x - b.x).abs().powi(2) + (a.y - b.y).abs().powi(2)).sqrt()
    }

    /// get zero or non-zero pixel the value at given coordinate
    fn get_at(&self, row: isize, col: isize, rows: usize, cols: usize, data: &[usize]) -> usize {
        if row < 0 || col < 0 || row >= rows as isize || col >= cols as isize {
            0
        } else {
            let idx = row as usize * cols + col as usize;
            data.get(idx)
                .map(|i| if *i == 0 { 0 } else { 1 })
                .unwrap_or_else(|| 0)
        }
    }

    /// translate point in positive x,y to either side of (0,0)
    fn xy_translate(&self, p: Vec2, rows: usize, cols: usize) -> Vec2 {
        Vec2::new(
            p.x - (cols as f32 / 2. - 1.0),
            -p.y + (rows as f32 / 2. - 1.0),
        )
    }

    /// Translate vector of points in positive x,y to either side of (0,0)
    pub fn translate_vec(&self, v: Vec<Vec2>, rows: usize, cols: usize) -> Vec<Vec2> {
        v.into_iter()
            .map(|p| self.xy_translate(p, rows, cols))
            .collect()
    }
}

impl TryFrom<DynamicImage> for Edges {
    type Error = error::Error;
    fn try_from(i: DynamicImage) -> Result<Edges, error::Error> {
        Ok(Edges { i })
    }
}

impl TryFrom<&DynamicImage> for Edges {
    type Error = error::Error;
    fn try_from(i: &DynamicImage) -> Result<Edges, error::Error> {
        Ok(Edges { i: i.clone() })
    }
}
