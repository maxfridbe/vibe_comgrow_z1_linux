import re
import os

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Generic function to replace variables in a string scope
    def replace_vars(text, is_test=False):
        t = text
        t = re.sub(r'\bb_enabled\b', 'config.boundary_enabled', t)
        t = re.sub(r'\bbw\b', 'config.boundary_w', t)
        t = re.sub(r'\bbh\b', 'config.boundary_h', t)
        t = re.sub(r'\bbx\b', 'config.boundary_x', t)
        t = re.sub(r'\bby\b', 'config.boundary_y', t)
        t = re.sub(r'\bscl\b', 'config.scale', t)
        t = re.sub(r'\bl_fid\b', 'config.img_low_fidelity', t)
        t = re.sub(r'\bh_fid\b', 'config.img_high_fidelity', t)
        t = re.sub(r'\bl_per_mm\b', 'config.img_lines_per_mm', t)
        
        # In test files, the original code used `&pwr.to_string()` where pwr = g.power / 10.0
        # and spd = g.feed_rate / 10.0
        # If is_test is True, the replacements should be (config.power / 10.0) and (config.feed_rate / 10.0)
        # Actually, if I look at the usages, they are usually in `replace("{POWER}", &pwr.to_string())`
        # Let's just do `(config.power / 10.0)` for test files. Wait, if it's used directly like `pwr`, 
        # it might need `(config.power / 10.0)`. In the original code they were `f32`, so `config.power / 10.0` works.
        if is_test:
            t = re.sub(r'\bpwr\b', '(config.power / 10.0)', t)
            t = re.sub(r'\bspd\b', '(config.feed_rate / 10.0)', t)
        else:
            t = re.sub(r'\bpwr\b', 'config.power', t)
            t = re.sub(r'\bspd\b', 'config.feed_rate', t)
            
        t = re.sub(r'\bpas\b', 'config.passes', t)
        return t

    if "ui_image.rs" in filepath:
        # Pattern 1:
        # let (pwr, spd, scl, pas, b_enabled, bx, by, bw, bh, l_fid, h_fid, l_per_mm) = ( ... );
        p1 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas,\s*b_enabled,\s*bx,\s*by,\s*bw,\s*bh,\s*l_fid,\s*h_fid,\s*l_per_mm\)\s*=\s*\((.*?)\);', re.DOTALL)
        
        def rep1(match):
            return "let config = g.get_burn_config();"

        # Pattern 2:
        # let (pwr, spd, scl, pas, b_enabled, bx, by, bw, bh, l_fid, h_fid, l_per_mm) = { ... };
        p2 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas,\s*b_enabled,\s*bx,\s*by,\s*bw,\s*bh,\s*l_fid,\s*h_fid,\s*l_per_mm\)\s*=\s*\{([^}]*)\};', re.DOTALL)
        def rep2(match):
            return """let config = {
                                let mut g = state.lock().unwrap();
                                g.is_processing = true;
                                g.get_burn_config()
                            };"""

        # Pattern 3:
        # let (pwr, spd, scl, pas) = { ... };
        p3 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas\)\s*=\s*\{([^}]*)\};', re.DOTALL)
        def rep3(match):
            return """let config = {
                let g = state.lock().unwrap();
                g.get_burn_config()
            };"""

        # Wait, I also need to replace the variable usages below these blocks, but how far?
        # A simpler way: just replace the block, then globally replace the variables in the file.
        # This is safe if pwr, spd, etc. are uniquely used in these contexts.
        # But `scl` might be `font_scale` etc.? The names are very specific: pwr, spd, scl, pas, b_enabled, bx, by, bw, bh, l_fid, h_fid, l_per_mm.
        # Wait, `scl` is used in slider macros.
        
        content = p1.sub(rep1, content)
        content = p2.sub(rep2, content)
        content = p3.sub(rep3, content)
        content = replace_vars(content, is_test=False)

    elif "ui_text.rs" in filepath:
        p3 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas\)\s*=\s*\{([^}]*)\};', re.DOTALL)
        def rep3(match):
            return """let config = {
                let g = state.lock().unwrap();
                g.get_burn_config()
            };"""
        
        content = p3.sub(rep3, content)
        content = replace_vars(content, is_test=False)

    elif "ui_test.rs" in filepath:
        p1 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas,\s*b_enabled,\s*bx,\s*by,\s*bw,\s*bh\)\s*=\s*\{([^}]*)\};', re.DOTALL)
        def rep1(match):
            return """let config = {
                                            let g = state.lock().unwrap();
                                            g.get_burn_config()
                                        };"""

        p2 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas,\s*b_enabled,\s*bx,\s*by,\s*bw,\s*bh\)\s*=\s*\((.*?)\);', re.DOTALL)
        def rep2(match):
            return "let config = g.get_burn_config();"

        p3 = re.compile(r'let\s+\(pwr,\s*spd,\s*scl,\s*pas\)\s*=\s*\{([^}]*)\};', re.DOTALL)
        def rep3(match):
            return """let config = {
                let g = state.lock().unwrap();
                g.get_burn_config()
            };"""

        content = p1.sub(rep1, content)
        content = p2.sub(rep2, content)
        content = p3.sub(rep3, content)
        content = replace_vars(content, is_test=True)

    # Some sliders cast `pas` to f32. Let's fix that formatting.
    content = content.replace("config.passes as f32", "config.passes as f32")
    # if `pas as f32` was replaced with `config.passes as f32`, that's fine.

    with open(filepath, 'w') as f:
        f.write(content)

process_file('src/ui_image.rs')
process_file('src/ui_text.rs')
process_file('src/ui_test.rs')
