# Phase 4: Branch Protection Configuration & Automated Enforcement

## 1. Overview

To protect `main` and prevent unverified, broken, or bypassing commits from reaching production, this document specifies the exact branch protection settings and `gh api` CLI commands for **EchoGrid** (`Web3onInfra/EchoGrid`).

All component CI checks (Rust, Python, Soroban Contract, Web App, Link Checker) aggregate into a single unified required gate: **`ci-ok`**. Only pull requests where `ci-ok` succeeds are mergeable.

---

## 2. GitHub UI Branch Protection Settings

In **GitHub Repository Settings** -> **Branches** -> **Add branch ruleset** (or **Branch protection rule**) for `main`:

### Target Branch
- **Branch name pattern:** `main`

### Protection Rules
1. **Require a pull request before merging:**
   - **Required approvals:** `1`
   - **Dismiss stale pull request approvals when new commits are pushed:** Checked (`true`)
   - **Require review from Code Owners:** Unchecked (`false`)
   - **Restrict who can dismiss pull request reviews:** Unchecked
2. **Require status checks to pass before merging:**
   - **Require branches to be up to date before merging:** Checked (`strict: true`)
   - **Status checks that are required:**
     - Add `ci-ok`
3. **Do not allow bypassing the above settings:**
   - **Enforce for Administrators:** Checked (`enforce_admins: true`)
4. **Require linear history:**
   - Checked (`true`)
5. **Include administrators:**
   - Checked (`true`)

---

## 3. GitHub CLI / REST API Enforcement Commands

Run the following command using the GitHub CLI with repository administrative permissions:

```bash
# Configure strict branch protection on 'main' requiring 'ci-ok'
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  /repos/Web3onInfra/EchoGrid/branches/main/protection \
  --input - << 'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "ci-ok"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": true
}
EOF
```

### Verification Command
To verify that the branch protection rules are active:

```bash
gh api \
  -H "Accept: application/vnd.github+json" \
  /repos/Web3onInfra/EchoGrid/branches/main/protection/required_status_checks
```

Expected output:
```json
{
  "url": "https://api.github.com/repos/Web3onInfra/EchoGrid/branches/main/protection/required_status_checks",
  "strict": true,
  "contexts": [
    "ci-ok"
  ],
  "contexts_url": "https://api.github.com/repos/Web3onInfra/EchoGrid/branches/main/protection/required_status_checks/contexts"
}
```
