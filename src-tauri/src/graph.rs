//! Lays out commits into lanes for the history graph.
//!
//! Commits arrive newest first (children before parents). Each lane holds the
//! hash of the commit it is waiting for. A commit takes the lane that waits
//! for it, lanes that also waited for it merge into it, and its parents are
//! assigned lanes for the rows below.
//!
//! Each row is described by line segments in a small coordinate system:
//! x is a lane index, y is 0 (top of the row), 1 (the commit dot) or
//! 2 (bottom of the row). The frontend draws them as SVG.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Segment {
    pub x1: usize,
    pub y1: u8,
    pub x2: usize,
    pub y2: u8,
    pub color: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct GraphRow {
    /// Lane of the commit dot.
    pub lane: usize,
    pub color: usize,
    pub segments: Vec<Segment>,
    /// Number of lanes in use on this row, for sizing the drawing.
    pub width: usize,
}

#[derive(Clone)]
struct Lane {
    waiting_for: String,
    color: usize,
}

/// `commits` is a list of (hash, parent hashes), newest first.
pub fn layout(commits: &[(String, Vec<String>)]) -> Vec<GraphRow> {
    let mut lanes: Vec<Option<Lane>> = Vec::new();
    let mut next_color = 0;
    let mut new_color = || {
        next_color += 1;
        next_color - 1
    };

    commits
        .iter()
        .map(|(hash, parents)| {
            let before = lanes.clone();
            let waiting: Vec<usize> = before
                .iter()
                .enumerate()
                .filter(|(_, l)| l.as_ref().is_some_and(|l| &l.waiting_for == hash))
                .map(|(i, _)| i)
                .collect();

            // The commit takes the leftmost lane waiting for it, or a free one
            // if nothing points here yet (a branch tip).
            let (lane, color) = match waiting.first() {
                Some(&i) => (i, before[i].as_ref().unwrap().color),
                None => (free_slot(&lanes), new_color()),
            };

            let mut segments = Vec::new();
            for &i in &waiting {
                let c = before[i].as_ref().unwrap().color;
                segments.push(Segment {
                    x1: i,
                    y1: 0,
                    x2: lane,
                    y2: 1,
                    color: c,
                });
                lanes[i] = None;
            }
            if lane >= lanes.len() {
                lanes.resize(lane + 1, None);
            }

            // First parent continues in this lane; others join a lane already
            // waiting for them or get a new one.
            match parents.first() {
                Some(first) => {
                    lanes[lane] = Some(Lane {
                        waiting_for: first.clone(),
                        color,
                    });
                    segments.push(Segment {
                        x1: lane,
                        y1: 1,
                        x2: lane,
                        y2: 2,
                        color,
                    });
                }
                None => lanes[lane] = None,
            }
            for parent in parents.iter().skip(1) {
                let existing = lanes
                    .iter()
                    .position(|l| l.as_ref().is_some_and(|l| &l.waiting_for == parent));
                let (target, c) = match existing {
                    Some(j) => (j, lanes[j].as_ref().unwrap().color),
                    None => {
                        let j = free_slot(&lanes);
                        let c = new_color();
                        if j >= lanes.len() {
                            lanes.resize(j + 1, None);
                        }
                        lanes[j] = Some(Lane {
                            waiting_for: parent.clone(),
                            color: c,
                        });
                        (j, c)
                    }
                };
                segments.push(Segment {
                    x1: lane,
                    y1: 1,
                    x2: target,
                    y2: 2,
                    color: c,
                });
            }

            // Lanes untouched by this commit pass straight through.
            for (i, l) in before.iter().enumerate() {
                if let (Some(b), Some(Some(a))) = (l, lanes.get(i)) {
                    if !waiting.contains(&i) && b.waiting_for == a.waiting_for {
                        segments.push(Segment {
                            x1: i,
                            y1: 0,
                            x2: i,
                            y2: 2,
                            color: b.color,
                        });
                    }
                }
            }

            while matches!(lanes.last(), Some(None)) {
                lanes.pop();
            }
            let width = before.len().max(lanes.len()).max(lane + 1);
            GraphRow {
                lane,
                color,
                segments,
                width,
            }
        })
        .collect()
}

fn free_slot(lanes: &[Option<Lane>]) -> usize {
    lanes
        .iter()
        .position(Option::is_none)
        .unwrap_or(lanes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hash: &str, parents: &[&str]) -> (String, Vec<String>) {
        (hash.into(), parents.iter().map(|p| p.to_string()).collect())
    }

    fn straight(x: usize, color: usize) -> Segment {
        Segment {
            x1: x,
            y1: 0,
            x2: x,
            y2: 2,
            color,
        }
    }

    #[test]
    fn linear_history_stays_in_one_lane() {
        let rows = layout(&[c("c", &["b"]), c("b", &["a"]), c("a", &[])]);
        assert!(rows.iter().all(|r| r.lane == 0 && r.width == 1));
        // The root commit has an incoming line but nothing below it.
        assert_eq!(
            rows[2].segments,
            vec![Segment {
                x1: 0,
                y1: 0,
                x2: 0,
                y2: 1,
                color: 0
            }]
        );
    }

    #[test]
    fn branch_and_merge() {
        //   m      merge of x into main
        //   |\
        //   b |    main
        //   | x    feature
        //   |/
        //   a
        let rows = layout(&[
            c("m", &["b", "x"]),
            c("b", &["a"]),
            c("x", &["a"]),
            c("a", &[]),
        ]);
        assert_eq!(rows[0].lane, 0);
        assert!(rows[0].segments.contains(&Segment {
            x1: 0,
            y1: 1,
            x2: 1,
            y2: 2,
            color: 1
        }));
        assert_eq!(rows[1].lane, 0);
        assert!(
            rows[1].segments.contains(&straight(1, 1)),
            "feature lane passes by b"
        );
        assert_eq!(rows[2].lane, 1);
        assert!(
            rows[2].segments.contains(&straight(0, 0)),
            "main lane passes by x"
        );
        // Both lanes wait for a, so they meet at its dot.
        assert_eq!(rows[3].lane, 0);
        assert!(rows[3].segments.contains(&Segment {
            x1: 1,
            y1: 0,
            x2: 0,
            y2: 1,
            color: 1
        }));
        assert_eq!(rows[3].width, 2);
    }

    #[test]
    fn unrelated_tips_get_their_own_lanes() {
        let rows = layout(&[c("t1", &["a"]), c("t2", &["a"]), c("a", &[])]);
        assert_eq!((rows[0].lane, rows[1].lane, rows[2].lane), (0, 1, 0));
        assert_ne!(rows[0].color, rows[1].color);
    }

    #[test]
    fn freed_lanes_are_reused() {
        // x branches off and ends at a root; the later tip y reuses its lane.
        let rows = layout(&[
            c("m", &["b", "x"]),
            c("x", &[]),
            c("y", &["b"]),
            c("b", &[]),
        ]);
        assert_eq!(rows[1].lane, 1);
        assert_eq!(rows[2].lane, 1, "lane 1 was freed by the root x");
    }
}
