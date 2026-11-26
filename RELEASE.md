# SwiftTray v1.0.2 — release operator checklist

**Status:** prepared source candidate, not published. Last confirmed public GitHub release: `v1.0.1`.

## Never skip the release gates

1. Use a clean Windows 11 x64 machine with the Rust MSVC toolchain, C++ Build Tools, and Windows SDK.
2. Run `powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-Release-Candidate.ps1 -BuildMSIX` from the repository root. This validates version, runs locked tests, builds EXE/sha256 and stages an UNSIGNED MSIX.
3. Verify no `SWIFTTRAY_EXPERIMENTAL_BUILD` flag is embedded in the final EXE, and review the two SHA256 values. Never publish an experimental EXE.
4. Run the **separate QA archive's isolated updater fixture** before checking a real release path. Also test failure recovery and non-writable folders, after backing up any real user installation.
5. Verify portable EXE on Windows 11 and Windows 10 (prefer 22H2). Known testing observations: elevated foreground apps may block shortcuts; 10 VM slider may look choppy while audio remains correct.
6. Test `Launch at startup` **inside a properly signed/test-installed MSIX**: enable, reboot/sign out, confirm tray; disable and confirm; test Windows Startup Apps override. Confirm `Open Store updates` opens Store. **Do not test by overwriting the production Store package**.
7. Review changes against the last published v1.0.1, especially audio/shortcut files and store identity; run a malware/security check as appropriate. Confirm update security limitations (SHA256 stored in same release, no independent publisher signature).
8. Commit/push the release-candidate source after tests pass. **Do not push a tag before the decision to draft the release.**
9. Create and push exactly `v1.0.2` from the verified commit: the GitHub Actions tag job validates versions and creates a **DRAFT** release with `SwiftTray.exe` and `SwiftTray.sha256`; manually inspect/checksum the downloaded assets. Draft is not served by `releases/latest`.
10. Publish the draft only after acceptance. Existing `v1.0.1` installations may need a **manual, one-time upgrade** if they lack the new updater. Do not claim automatic migration from old EXE without checking.
11. Prepare an MSIX submission in Partner Center from the staged package using the approved package identity. Store certification and rollout occur separately from GitHub; never alter package Name/Publisher/Application Id/Executable.

## Why it is not ready for a live release yet

Source archive and version metadata are prepared, but final Windows compile of **this cleaned candidate**, real self-replacement/rollback test, and packaged Store behavior are not yet verified. This is not an installer and not a signed package.

See the separate `SwiftTray-v1.0.2-QA-Kit.zip` for the offline updater test and tomorrow handoff checklist.
