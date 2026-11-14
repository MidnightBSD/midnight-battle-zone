use std::cmp::Ordering;
use std::collections::BinaryHeap;
use glam::Vec2;

#[derive(Copy, Clone, PartialEq)]
struct PathNode {
    f_score: f32,
    index: usize,
}

impl Eq for PathNode {}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn line_intersects_obstacles(p1: Vec2, p2: Vec2, obstacles: &[(Vec2, f32)], clearance: f32) -> bool {
    let d = p2 - p1;
    let len_sq = d.length_squared();

    for &(obs_center, obs_radius) in obstacles {
        let threshold = obs_radius + clearance;
        let threshold_sq = threshold * threshold;

        if len_sq < 0.0001 {
            if (p1 - obs_center).length_squared() < threshold_sq {
                return true;
            }
            continue;
        }

        let t = ((obs_center - p1).dot(d) / len_sq).clamp(0.0, 1.0);
        let projection = p1 + d * t;
        if (obs_center - projection).length_squared() < threshold_sq {
            return true;
        }
    }
    false
}

pub fn find_path(
    start: Vec2,
    goal: Vec2,
    map_width: f32,
    map_height: f32,
    obstacles: &[(Vec2, f32)],
) -> Vec<Vec2> {
    let clearance = 0.8;

    // Check direct line of sight first
    if !line_intersects_obstacles(start, goal, obstacles, clearance) {
        return vec![goal];
    }

    // Grid configuration
    let grid_step: f32 = 2.0;
    let cols = ((map_width / grid_step).ceil() as usize).max(2);
    let rows = ((map_height / grid_step).ceil() as usize).max(2);
    let total_cells = cols * rows;

    let to_cell_coords = |pos: Vec2| -> (usize, usize) {
        let cx = ((pos.x / grid_step).floor() as isize).clamp(0, (cols - 1) as isize) as usize;
        let cy = ((pos.y / grid_step).floor() as isize).clamp(0, (rows - 1) as isize) as usize;
        (cx, cy)
    };

    let to_world_pos = |cx: usize, cy: usize| -> Vec2 {
        Vec2::new(
            (cx as f32 + 0.5) * grid_step,
            (cy as f32 + 0.5) * grid_step,
        )
    };

    let is_walkable = |cx: usize, cy: usize| -> bool {
        let p = to_world_pos(cx, cy);
        for &(obs_center, obs_radius) in obstacles {
            let threshold = obs_radius + clearance;
            if (p - obs_center).length_squared() < threshold * threshold {
                return false;
            }
        }
        true
    };

    let (start_cx, start_cy) = to_cell_coords(start);
    let (mut goal_cx, mut goal_cy) = to_cell_coords(goal);

    // If goal is not walkable, search outward for nearest walkable cell
    if !is_walkable(goal_cx, goal_cy) {
        let mut best_dist_sq = f32::MAX;
        let mut alt_found = false;
        let search_radius = 4isize;
        for dy in -search_radius..=search_radius {
            for dx in -search_radius..=search_radius {
                let ncx = goal_cx as isize + dx;
                let ncy = goal_cy as isize + dy;
                if ncx >= 0 && ncx < cols as isize && ncy >= 0 && ncy < rows as isize {
                    let (ux, uy) = (ncx as usize, ncy as usize);
                    if is_walkable(ux, uy) {
                        let wpos = to_world_pos(ux, uy);
                        let d_sq = (wpos - goal).length_squared();
                        if d_sq < best_dist_sq {
                            best_dist_sq = d_sq;
                            goal_cx = ux;
                            goal_cy = uy;
                            alt_found = true;
                        }
                    }
                }
            }
        }
        if !alt_found {
            // Cannot find alternative walkable cell near goal
            return vec![goal];
        }
    }

    let start_idx = start_cy * cols + start_cx;
    let goal_idx = goal_cy * cols + goal_cx;

    if start_idx == goal_idx {
        return vec![goal];
    }

    let heuristic = |cx: usize, cy: usize| -> f32 {
        let dx = (cx as f32 - goal_cx as f32).abs();
        let dy = (cy as f32 - goal_cy as f32).abs();
        let diagonal = dx.min(dy);
        let straight = (dx - dy).abs();
        (diagonal * 1.41421356 + straight) * grid_step
    };

    let mut g_score = vec![f32::INFINITY; total_cells];
    let mut came_from = vec![usize::MAX; total_cells];
    let mut closed = vec![false; total_cells];
    let mut open_set = BinaryHeap::new();

    g_score[start_idx] = 0.0;
    open_set.push(PathNode {
        f_score: heuristic(start_cx, start_cy),
        index: start_idx,
    });

    let mut best_idx = start_idx;
    let mut best_h = heuristic(start_cx, start_cy);
    let mut expanded_count = 0;
    const MAX_EXPANDED_NODES: usize = 1200;

    let neighbors_offset: [(isize, isize, f32); 8] = [
        (1, 0, 1.0),
        (-1, 0, 1.0),
        (0, 1, 1.0),
        (0, -1, 1.0),
        (1, 1, 1.41421356),
        (-1, 1, 1.41421356),
        (1, -1, 1.41421356),
        (-1, -1, 1.41421356),
    ];

    while let Some(PathNode { index: current, .. }) = open_set.pop() {
        if current == goal_idx {
            best_idx = current;
            break;
        }

        if closed[current] {
            continue;
        }
        closed[current] = true;
        expanded_count += 1;

        let cur_cx = current % cols;
        let cur_cy = current / cols;
        let cur_g = g_score[current];

        let cur_h = heuristic(cur_cx, cur_cy);
        if cur_h < best_h {
            best_h = cur_h;
            best_idx = current;
        }

        if expanded_count >= MAX_EXPANDED_NODES {
            break;
        }

        for &(dx, dy, step_mult) in &neighbors_offset {
            let ncx = cur_cx as isize + dx;
            let ncy = cur_cy as isize + dy;

            if ncx < 0 || ncx >= cols as isize || ncy < 0 || ncy >= rows as isize {
                continue;
            }

            let nx = ncx as usize;
            let ny = ncy as usize;
            let n_idx = ny * cols + nx;

            if closed[n_idx] {
                continue;
            }

            // Diagonal corner-cutting prevention
            if dx != 0 && dy != 0 {
                let adj1_walkable = is_walkable(cur_cx, ny);
                let adj2_walkable = is_walkable(nx, cur_cy);
                if !adj1_walkable || !adj2_walkable {
                    continue;
                }
            }

            if !is_walkable(nx, ny) {
                continue;
            }

            let tentative_g = cur_g + step_mult * grid_step;
            if tentative_g < g_score[n_idx] {
                g_score[n_idx] = tentative_g;
                came_from[n_idx] = current;
                let f = tentative_g + heuristic(nx, ny);
                open_set.push(PathNode {
                    f_score: f,
                    index: n_idx,
                });
            }
        }
    }

    // Reconstruct path
    let mut raw_path = Vec::new();
    let mut curr = best_idx;
    while curr != usize::MAX {
        let cx = curr % cols;
        let cy = curr / cols;
        raw_path.push(to_world_pos(cx, cy));
        if curr == start_idx {
            break;
        }
        curr = came_from[curr];
    }
    raw_path.reverse();

    if raw_path.is_empty() {
        return vec![goal];
    }

    // Replace first waypoint with start and last with goal if we reached goal
    if best_idx == goal_idx {
        if let Some(last) = raw_path.last_mut() {
            *last = goal;
        }
    }

    // Smooth path using line of sight checks
    let mut smoothed = Vec::new();
    let mut cur_i = 0;
    while cur_i < raw_path.len() {
        let p_cur = raw_path[cur_i];
        let mut next_i = cur_i + 1;
        let mut furthest_i = cur_i + 1;

        while next_i < raw_path.len() {
            let p_next = raw_path[next_i];
            if !line_intersects_obstacles(p_cur, p_next, obstacles, clearance) {
                furthest_i = next_i;
                next_i += 1;
            } else {
                break;
            }
        }

        if furthest_i < raw_path.len() {
            smoothed.push(raw_path[furthest_i]);
            cur_i = furthest_i;
        } else {
            if furthest_i == raw_path.len() && !smoothed.contains(&raw_path[raw_path.len() - 1]) {
                smoothed.push(raw_path[raw_path.len() - 1]);
            }
            break;
        }
    }

    if smoothed.is_empty() {
        smoothed.push(goal);
    } else if let Some(last) = smoothed.last_mut() {
        if best_idx == goal_idx {
            *last = goal;
        }
    }

    smoothed
}

pub fn compute_separation(pos: Vec2, radius: f32, neighbors: &[(Vec2, f32)]) -> Vec2 {
    let mut force = Vec2::ZERO;
    let effective_radius = if radius > 0.0 { radius } else { 0.8 };

    for &(neighbor_pos, neighbor_radius) in neighbors {
        let diff = pos - neighbor_pos;
        let dist = diff.length();
        let target_dist = effective_radius + if neighbor_radius > 0.0 { neighbor_radius } else { 0.8 };

        if dist < target_dist {
            if dist > 0.0001 {
                let overlap = target_dist - dist;
                let dir = diff / dist;
                force += dir * (overlap / target_dist);
            } else {
                force += Vec2::new(1.0, 0.0);
            }
        }
    }

    let max_force = 1.5;
    if force.length_squared() > max_force * max_force {
        force = force.normalize() * max_force;
    }

    force
}
