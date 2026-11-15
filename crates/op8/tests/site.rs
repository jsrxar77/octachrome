//! The public site (guide/ → site/): it builds, and everything in it holds together (docs/features.md §G).
use std::process::Command;

fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

// F-G05: scripts/build-site.py makes site/ from the guide, the twin and the button map, and scripts/check-site.py finds nothing wrong
// in it: no broken link, anchor or image, every page with a title, a description and one heading, no template marker left, the brand
// page out of the sitemap. (The placeholders for the site's own address are warnings, not errors.)
#[test]
fn f_g05_the_site_builds_and_holds_together() {
    let Some(py) = python() else {
        eprintln!("no python on this machine: the site is not checked here");
        return;
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for script in ["scripts/build-site.py", "scripts/check-site.py"] {
        let out = Command::new(py).arg(script).current_dir(&root).output().expect("runs");
        let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{script} failed:\n{text}");
    }
    let site = root.join("site");
    for f in ["index.html", "manual.html", "404.html", "robots.txt", "sitemap.xml", "og.png", "twin/op8-twin.html", "brand.html"] {
        assert!(site.join(f).is_file(), "{f} is in the site");
    }
    let manual = std::fs::read_to_string(site.join("manual.html")).unwrap();
    assert!(manual.contains("The unit, part by part") && manual.contains("id=\"r17\""), "the manual is whole");
    let robots = std::fs::read_to_string(site.join("robots.txt")).unwrap();
    assert!(robots.contains("Disallow: /brand.html"));
}
