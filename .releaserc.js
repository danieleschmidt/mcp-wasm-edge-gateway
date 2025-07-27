// Semantic Release configuration for MCP WASM Edge Gateway

module.exports = {
  branches: [
    'main',
    { name: 'develop', prerelease: true },
    { name: 'alpha', prerelease: true },
    { name: 'beta', prerelease: true }
  ],
  
  plugins: [
    // Analyze commits to determine version bump
    '@semantic-release/commit-analyzer',
    
    // Generate release notes
    '@semantic-release/release-notes-generator',
    
    // Update CHANGELOG.md
    [
      '@semantic-release/changelog',
      {
        changelogFile: 'CHANGELOG.md',
        changelogTitle: '# Changelog\n\nAll notable changes to this project will be documented in this file.'
      }
    ],
    
    // Update Cargo.toml version
    [
      '@semantic-release/exec',
      {
        prepareCmd: 'sed -i "s/^version = .*/version = \\"${nextRelease.version}\\"/" Cargo.toml'
      }
    ],
    
    // Update package.json version
    '@semantic-release/npm',
    
    // Create GitHub release
    [
      '@semantic-release/github',
      {
        assets: [
          {
            path: 'releases/${nextRelease.version}/gateway-linux-x86_64',
            name: 'gateway-linux-x86_64',
            label: 'Linux x86_64 binary'
          },
          {
            path: 'releases/${nextRelease.version}/gateway-linux-aarch64',
            name: 'gateway-linux-aarch64',
            label: 'Linux ARM64 binary'
          },
          {
            path: 'releases/${nextRelease.version}/gateway-windows-x86_64.exe',
            name: 'gateway-windows-x86_64.exe',
            label: 'Windows x86_64 binary'
          },
          {
            path: 'releases/${nextRelease.version}/gateway-wasm-web.tar.gz',
            name: 'gateway-wasm-web.tar.gz',
            label: 'WASM Web package'
          },
          {
            path: 'releases/${nextRelease.version}/gateway-wasm-node.tar.gz',
            name: 'gateway-wasm-node.tar.gz',
            label: 'WASM Node.js package'
          },
          {
            path: 'releases/${nextRelease.version}/checksums.txt',
            name: 'checksums.txt',
            label: 'SHA256 checksums'
          }
        ]
      }
    ],
    
    // Commit updated files
    [
      '@semantic-release/git',
      {
        assets: ['CHANGELOG.md', 'Cargo.toml', 'package.json', 'Cargo.lock'],
        message: 'chore(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}'
      }
    ]
  ],
  
  preset: 'conventionalcommits',
  
  releaseRules: [
    { type: 'feat', release: 'minor' },
    { type: 'fix', release: 'patch' },
    { type: 'perf', release: 'patch' },
    { type: 'revert', release: 'patch' },
    { type: 'docs', scope: 'README', release: 'patch' },
    { type: 'refactor', release: 'patch' },
    { type: 'style', release: false },
    { type: 'chore', release: false },
    { type: 'test', release: false },
    { breaking: true, release: 'major' }
  ],
  
  parserOpts: {
    noteKeywords: ['BREAKING CHANGE', 'BREAKING CHANGES', 'BREAKING']
  }
};