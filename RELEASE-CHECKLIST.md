# OpenClaw Rust Workspace - Release Checklist

## Pre-Release (Before GitHub Release)

### ✅ Code Complete
- [x] All tests passing (70 unit + 6 integration)
- [x] Clippy warnings: 0
- [x] Format check passing
- [x] Documentation complete

### ✅ Binaries Built
- [x] `mcp-server` (3.7MB) - `./target/release/mcp-server`
- [x] `openclaw_node_bridge.node` (5.1MB) - `./target/release/openclaw_node_bridge.node`

### ✅ Documentation
- [x] README.md - Main documentation
- [x] QUICKSTART.md - Quick start guide
- [x] FFI.md - Node.js API reference
- [x] CLAUDE.md - Claude Desktop integration
- [x] BENCHMARKS.md - Performance benchmarks
- [x] RELEASES.md - Release notes
- [x] CHANGELOG.md - Version history
- [x] CONTRIBUTING.md - Developer guide
- [x] SECURITY.md - Security policy

### ✅ Project Metadata
- [x] LICENSE (MIT OR Apache-2.0)
- [x] Cargo.toml with proper metadata
- [x] .github/workflows/ci.yml - GitHub Actions CI
- [x] rustfmt.toml - Code formatting
- [x] clippy.toml - Linting rules

## GitHub Release (Requires GitHub Token)

### Steps to Release

1. **Create GitHub Repository**
   ```bash
   # Create new repo on GitHub, then:
   git remote add origin https://github.com/YOUR_USERNAME/openclaw-rs.git
   git push -u origin master
   ```

2. **Push Tags**
   ```bash
   git tag v0.2.1
   git push origin v0.2.1
   ```

3. **Create GitHub Release**
   - Go to: https://github.com/YOUR_USERNAME/openclaw-rs/releases/new
   - Tag: v0.2.1
   - Title: OpenClaw Rust Workspace v0.2.1
   - Copy content from RELEASES.md

4. **Upload Binaries** (Optional - for manual download)
   - Attach `./target/release/mcp-server`
   - Attach `./target/release/openclaw_node_bridge.node`

5. **Enable GitHub Actions**
   - Workflows should auto-run on push
   - Check Actions tab for results

## Post-Release

### Announcements
- [ ] Post to relevant communities
- [ ] Update documentation if needed
- [ ] Monitor for issue reports

## Version History

| Version | Date | Status |
|---------|------|--------|
| v0.2.1 | 2026-04-17 | Ready to release |
| v0.2.0 | 2026-04-16 | Released |
| v0.1.0 | 2026-04-06 | Released |

## Quick Release Commands

```bash
# 1. Ensure on master and clean
git checkout master
git status  # Should be clean

# 2. Update version if needed
# Edit Cargo.toml files

# 3. Commit all changes
git add -A
git commit -m "chore: prep for v0.2.1 release"

# 4. Create and push tag
git tag v0.2.1
git push origin master --tags

# 5. After GitHub Actions passes, create release
gh release create v0.2.1 \
  --title "OpenClaw Rust Workspace v0.2.1" \
  --notes-file RELEASES.md
```
