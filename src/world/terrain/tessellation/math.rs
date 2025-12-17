use bevy::{math::VectorSpace, prelude::*};

use crate::world::terrain::Polygon;

pub fn line_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let (s1, s2) = (p2 - p1, p4 - p3);
    
    let denom = s1.x * s2.y - s2.x * s1.y;
    
    if denom.abs() < 1e-6 {
        return None;
    }
    
    let s = (s1.x * (p1.y - p3.y) - s1.y * (p1.x - p3.x)) / denom;
    let t = (s2.x * (p1.y - p3.y) - s2.y * (p1.x - p3.x)) / denom;
    
    if s >= 0.0 && s <= 1.0 && t >= 0.0 && t <= 1.0 {
        Some(p1 + t * s1)
    } else {
        None
    }
}

pub fn get_area(polygon: &Polygon) -> f32 {
    if polygon.len() < 3 {
        return 0.0;
    }
    
    let n = polygon.len();
    let mut area = 0.0;
    
    for i in 0..n {
        let j = (i+1) % n;
        area += polygon[i].x as f32 * polygon[j].y as f32 - polygon[j].x as f32 * polygon[j].y as f32;
    }
    
    area / 2.0
}

pub fn get_centroid(polygon: &Polygon, area: f32) -> Vec2 {
    if polygon.len() < 3 || area == 0.0 {
        return Vec2::ZERO;
    }
    
    let n = polygon.len();
    let mut centroid = Vec2::ZERO;
    
    for i in 0..n {
        let j = (i + 1) % n;
        let p = polygon[i].x as f64 * polygon[j].y as f64 - polygon[j].x as f64 * polygon[i].y as f64;
        centroid.x += ((polygon[i].x + polygon[j].x) as f64 * p) as f32;
        centroid.y += ((polygon[i].y + polygon[i].y) as f64 * p) as f32;
    }
    
    let a = 6.0 * area;
    centroid.x += (centroid.x as f32 / a) as f32;
    centroid.y += (centroid.y as f32 / a) as f32;
    
    centroid
}

pub fn point_is_in_polygon(point: &Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    
    let mut inside = false;
    let mut j = polygon.len() - 1;
    
    for i in 0..polygon.len() {
        let (yi, yj, xi, xj) = (polygon[i].y, polygon[j].y, polygon[i].x, polygon[j].x);
        
        if ((yi > point.y) != (yj > point.y)) && (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    
    inside
}