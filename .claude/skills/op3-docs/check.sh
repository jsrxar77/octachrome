#!/bin/sh
# Checks the rules of the op3-docs skill on docs/. Run from anywhere in the repo.
# Exit status 0 when everything holds, 1 otherwise.

cd "$(git rev-parse --show-toplevel)" || exit 2
fail=0
ok()  { printf 'ok    %s\n' "$1"; }
bad() { printf 'FAIL  %s\n' "$1"; fail=1; }

# 1. docs/ holds exactly the six documents.
want="architecture.md buttons.md design.md features.md instruments.md roadmap.md"
have=$(ls docs | tr '\n' ' ' | sed 's/ $//')
if [ "$have" = "$want" ]; then ok "docs/ holds exactly the six documents"; else bad "docs/ holds: $have"; fi

# 2. No reference to a name that no longer exists.
stale=$(grep -rIn -E "HANDOVER|BOTONES|INSTRUMENTOS|ROADMAP\.md|docs/DESIGN\.md|docs/op3-twin|docs/sim-screens" \
  --include="*.md" --include="*.rs" --include="*.html" --include="*.yml" --include="*.sh" --include="*.toml" \
  --exclude-dir=.git --exclude-dir=target --exclude-dir=.claude . 2>/dev/null)
if [ -z "$stale" ]; then ok "no reference to removed documents or moved files"; else bad "stale references:"; echo "$stale"; fi

# 3. Links between documents resolve.
broken=$(for f in docs/*.md; do
  grep -o '](\([a-z0-9_-]*\.md\)' "$f" | sed 's/](//' | sort -u | while read -r l; do
    [ -f "docs/$l" ] || echo "$f -> $l"
  done
done)
if [ -z "$broken" ]; then ok "links between documents resolve"; else bad "broken links:"; echo "$broken"; fi

# 4. Every feature that is not manual review (R) has its test, and every ID in the tests is in features.md.
missing=$(grep -E '^\| F-[A-Z][0-9]{2} ' docs/features.md | grep -v '| R |$' | grep -o '^| F-[A-Z][0-9]\{2\}' | sed 's/| //' | while read -r id; do
  grep -rqs "$id" crates --include="*.rs" || echo "$id"
done)
if [ -z "$missing" ]; then ok "every automated feature has a test carrying its ID"; else bad "features with no test: $missing"; fi
orphans=$(grep -rhoE 'F-[A-Z][0-9]{2}' crates --include="*.rs" | sort -u | while read -r id; do
  grep -q "| $id " docs/features.md || echo "$id"
done)
if [ -z "$orphans" ]; then ok "every ID in the tests is in features.md"; else bad "IDs in tests but not in features.md: $orphans"; fi

# 5. Documents are in English (no Spanish accents or inverted marks).
spanish=$(grep -n '[áéíóúñ¿¡]' docs/*.md | head -5)
if [ -z "$spanish" ]; then ok "documents are in English"; else bad "Spanish characters found:"; echo "$spanish"; fi

exit $fail
