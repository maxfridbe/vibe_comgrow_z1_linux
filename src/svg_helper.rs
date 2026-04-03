pub fn load_svg_as_gcode(path: &str, scale: f32, off_x: f32, off_y: f32) -> Result<(String, f32, f32), Box<dyn std::error::Error + Send + Sync>> {
    let opt = usvg::Options::default();
    let data = std::fs::read(path)?;
    let tree = usvg::Tree::from_data(&data, &opt)?;
    
    let mut gcode = String::new();
    let mut max_x = off_x;
    let mut max_y = off_y;
    
    // In usvg 0.47, tree.root() returns &Group
    for node in tree.root().children() {
        if let usvg::Node::Path(path) = node {
            let transform = path.abs_transform();
            let mut curr_x = 0.0;
            let mut curr_y = 0.0;
            
            for segment in path.data().segments() {
                match segment {
                    usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                        let mut pt = p;
                        transform.map_point(&mut pt);
                        let px = off_x + (pt.x * scale);
                        let py = off_y + (pt.y * scale);
                        gcode.push_str(&format!("M5\nG0 X{:.2} Y{:.2}\nM4\n", px, py));
                        if px > max_x { max_x = px; }
                        if py > max_y { max_y = py; }
                        curr_x = pt.x;
                        curr_y = pt.y;
                    }
                    usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                        let mut pt = p;
                        transform.map_point(&mut pt);
                        let px = off_x + (pt.x * scale);
                        let py = off_y + (pt.y * scale);
                        gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                        if px > max_x { max_x = px; }
                        if py > max_y { max_y = py; }
                        curr_x = pt.x;
                        curr_y = pt.y;
                    }
                    usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p) => {
                        let mut cp1 = p1;
                        let mut cp2 = p2;
                        let mut end_p = p;
                        transform.map_point(&mut cp1);
                        transform.map_point(&mut cp2);
                        transform.map_point(&mut end_p);

                        let bezier = lyon_geom::CubicBezierSegment {
                            from: lyon_geom::point(curr_x, curr_y),
                            ctrl1: lyon_geom::point(cp1.x, cp1.y),
                            ctrl2: lyon_geom::point(cp2.x, cp2.y),
                            to: lyon_geom::point(end_p.x, end_p.y),
                        };

                        for point in bezier.flattened(0.1) {
                            let px = off_x + (point.x * scale);
                            let py = off_y + (point.y * scale);
                            gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                            if px > max_x { max_x = px; }
                            if py > max_y { max_y = py; }
                        }
                        curr_x = end_p.x;
                        curr_y = end_p.y;
                    }
                    usvg::tiny_skia_path::PathSegment::QuadTo(p1, p) => {
                        let mut cp1 = p1;
                        let mut end_p = p;
                        transform.map_point(&mut cp1);
                        transform.map_point(&mut end_p);

                        let bezier = lyon_geom::QuadraticBezierSegment {
                            from: lyon_geom::point(curr_x, curr_y),
                            ctrl: lyon_geom::point(cp1.x, cp1.y),
                            to: lyon_geom::point(end_p.x, end_p.y),
                        };

                        for point in bezier.flattened(0.1) {
                            let px = off_x + (point.x * scale);
                            let py = off_y + (point.y * scale);
                            gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                            if px > max_x { max_x = px; }
                            if py > max_y { max_y = py; }
                        }
                        curr_x = end_p.x;
                        curr_y = end_p.y;
                    }
                    usvg::tiny_skia_path::PathSegment::Close => {}
                }
            }
        }
    }
    
    Ok((gcode, max_x, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_svg_flattening() {
        let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <circle cx="50" cy="50" r="40" />
        </svg>"#;
        
        let mut file = std::fs::File::create("test_circle.svg").unwrap();
        file.write_all(svg_data.as_bytes()).unwrap();
        
        let (gcode, _mx, _my) = load_svg_as_gcode("test_circle.svg", 1.0, 0.0, 0.0).unwrap();
        
        let line_count = gcode.lines().filter(|l| l.starts_with("G1")).count();
        println!("Circle G1 segments: {}", line_count);
        
        assert!(line_count > 10, "Circle should be flattened into multiple line segments, found only {}", line_count);
        
        std::fs::remove_file("test_circle.svg").unwrap();
    }
}
