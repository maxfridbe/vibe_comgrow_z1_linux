pub fn load_svg_as_gcode(path: &str, scale: f32, off_x: f32, off_y: f32, s_val: i32, f_val: i32) -> Result<(String, f32, f32, f32, f32), Box<dyn std::error::Error + Send + Sync>> {
    let opt = usvg::Options::default();
    let data = std::fs::read(path)?;
    let tree = usvg::Tree::from_data(&data, &opt)?;
    
    let mut gcode = String::new();
    let mut min_px: Option<f32> = None;
    let mut max_px: Option<f32> = None;
    let mut min_py: Option<f32> = None;
    let mut max_py: Option<f32> = None;

    let mut update_bounds = |x: f32, y: f32| {
        if min_px.is_none() || x < min_px.unwrap() { min_px = Some(x); }
        if max_px.is_none() || x > max_px.unwrap() { max_px = Some(x); }
        if min_py.is_none() || y < min_py.unwrap() { min_py = Some(y); }
        if max_py.is_none() || y > max_py.unwrap() { max_py = Some(y); }
    };
    
    for node in tree.root().children() {
        if let usvg::Node::Path(path) = node {
            let transform = path.abs_transform();
            let mut curr_x = 0.0;
            let mut curr_y = 0.0;
            let mut start_pt: Option<(f32, f32)> = None;
            
            for segment in path.data().segments() {
                match segment {
                    usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                        let mut pt = p;
                        transform.map_point(&mut pt);
                        let px = off_x + (pt.x * scale);
                        let py = off_y + (pt.y * scale);
                        gcode.push_str(&format!("M5\nG0 X{:.2} Y{:.2}\nM4 S{} F{}\n", px, py, s_val, f_val));
                        update_bounds(px, py);
                        curr_x = pt.x;
                        curr_y = pt.y;
                        start_pt = Some((pt.x, pt.y));
                    }
                    usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                        let mut pt = p;
                        transform.map_point(&mut pt);
                        let px = off_x + (pt.x * scale);
                        let py = off_y + (pt.y * scale);
                        gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                        update_bounds(px, py);
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
                            update_bounds(px, py);
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
                            update_bounds(px, py);
                        }
                        curr_x = end_p.x;
                        curr_y = end_p.y;
                    }
                    usvg::tiny_skia_path::PathSegment::Close => {
                        if let Some((sx, sy)) = start_pt {
                            let px = off_x + (sx * scale);
                            let py = off_y + (sy * scale);
                            gcode.push_str(&format!("G1 X{:.2} Y{:.2}\n", px, py));
                            update_bounds(px, py);
                            curr_x = sx;
                            curr_y = sy;
                        }
                    }
                }
            }
        }
    }
    
    Ok((gcode, min_px.unwrap_or(off_x), min_py.unwrap_or(off_y), max_px.unwrap_or(off_x), max_py.unwrap_or(off_y)))
}
