enum Op {
    MoveTo(f32, f32),
    LineTo(f32, f32),
}

pub fn load_svg_as_gcode(path: &str, scale: f32, fit: Option<(f32, f32)>, center_x: f32, center_y: f32, s_val: i32, f_val: i32) -> Result<(String, f32, f32, f32, f32), Box<dyn std::error::Error + Send + Sync>> {
    let opt = usvg::Options::default();
    let data = std::fs::read(path)?;
    let tree = usvg::Tree::from_data(&data, &opt)?;
    
    let mut min_orig_x: Option<f32> = None;
    let mut max_orig_x: Option<f32> = None;
    let mut min_orig_y: Option<f32> = None;
    let mut max_orig_y: Option<f32> = None;

    let mut update_orig_bounds = |x: f32, y: f32| {
        if min_orig_x.is_none() || x < min_orig_x.unwrap() { min_orig_x = Some(x); }
        if max_orig_x.is_none() || x > max_orig_x.unwrap() { max_orig_x = Some(x); }
        if min_orig_y.is_none() || y < min_orig_y.unwrap() { min_orig_y = Some(y); }
        if max_orig_y.is_none() || y > max_orig_y.unwrap() { max_orig_y = Some(y); }
    };
    
    let mut paths = Vec::new();
    fn extract_paths<'a>(group: &'a usvg::Group, paths: &mut Vec<&'a usvg::Path>) {
        for node in group.children() {
            match node {
                usvg::Node::Group(g) => extract_paths(g, paths),
                usvg::Node::Path(p) => paths.push(p),
                _ => {}
            }
        }
    }
    extract_paths(tree.root(), &mut paths);

    let mut ops = Vec::new();

    for path in paths {
        let transform = path.abs_transform();
        let mut curr_x = 0.0;
        let mut curr_y = 0.0;
        let mut start_pt: Option<(f32, f32)> = None;
        
        for segment in path.data().segments() {
            match segment {
                usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                    let mut pt = p;
                    transform.map_point(&mut pt);
                    ops.push(Op::MoveTo(pt.x, pt.y));
                    update_orig_bounds(pt.x, pt.y);
                    curr_x = pt.x;
                    curr_y = pt.y;
                    start_pt = Some((pt.x, pt.y));
                }
                usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                    let mut pt = p;
                    transform.map_point(&mut pt);
                    ops.push(Op::LineTo(pt.x, pt.y));
                    update_orig_bounds(pt.x, pt.y);
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
                        ops.push(Op::LineTo(point.x, point.y));
                        update_orig_bounds(point.x, point.y);
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
                        ops.push(Op::LineTo(point.x, point.y));
                        update_orig_bounds(point.x, point.y);
                    }
                    curr_x = end_p.x;
                    curr_y = end_p.y;
                }
                usvg::tiny_skia_path::PathSegment::Close => {
                    if let Some((sx, sy)) = start_pt {
                        ops.push(Op::LineTo(sx, sy));
                        update_orig_bounds(sx, sy);
                        curr_x = sx;
                        curr_y = sy;
                    }
                }
            }
        }
    }

    let orig_min_x = min_orig_x.unwrap_or(0.0);
    let orig_max_x = max_orig_x.unwrap_or(0.0);
    let orig_min_y = min_orig_y.unwrap_or(0.0);
    let orig_max_y = max_orig_y.unwrap_or(0.0);

    let orig_w = orig_max_x - orig_min_x;
    let orig_h = orig_max_y - orig_min_y;

    let mut final_scale = scale;
    if let Some((fit_w, fit_h)) = fit {
        if orig_w > 0.0 && orig_h > 0.0 {
            let sw = fit_w / orig_w;
            let sh = fit_h / orig_h;
            final_scale = sw.min(sh);
        }
    }

    let out_w = orig_w * final_scale;
    let out_h = orig_h * final_scale;

    let offset_x = center_x - out_w / 2.0 - orig_min_x * final_scale;
    let offset_y = center_y - out_h / 2.0 - orig_min_y * final_scale;

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

    for op in ops {
        match op {
            Op::MoveTo(x, y) => {
                let px = offset_x + (x * final_scale);
                let py = offset_y + (y * final_scale);
                gcode.push_str(&format!("{}\n{}\n{}\n", crate::gcode::CMD_LASER_OFF, crate::gcode::move_xy(px, py), crate::gcode::laser_on_dynamic_f(s_val as f32, f_val as f32)));
                update_bounds(px, py);
            }
            Op::LineTo(x, y) => {
                let px = offset_x + (x * final_scale);
                let py = offset_y + (y * final_scale);
                gcode.push_str(&format!("{}\n", crate::gcode::burn_s(px, py, s_val as f32)));
                update_bounds(px, py);
            }
        }
    }

    Ok((gcode, min_px.unwrap_or(center_x), min_py.unwrap_or(center_y), max_px.unwrap_or(center_x), max_py.unwrap_or(center_y)))
}
