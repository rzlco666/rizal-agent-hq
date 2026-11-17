import { chromium, defineConfig, devices } from '@playwright/test';
import { existsSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

function availableChromium(): string | undefined {
  const expected = chromium.executablePath();
  if (existsSync(expected)) return undefined;
  const root = join(homedir(), '.cache', 'ms-playwright');
  if (!existsSync(root)) return undefined;
  const revisions = readdirSync(root)
    .filter((name) => name.startsWith('chromium-'))
    .sort()
    .reverse();
  for (const revision of revisions) {
    const binary = join(root, revision, 'chrome-linux64', 'chrome');
    if (existsSync(binary)) return binary;
  }
  return undefined;
}

const executablePath = availableChromium();

export default defineConfig({
  testDir: 'tests',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://127.0.0.1:8901',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    launchOptions: executablePath ? { executablePath } : {}
  },
  projects: [
    {
      name: 'chromium',
      testIgnore: /tests\/perf\//,
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } }
    },
    {
      name: 'phone',
      testIgnore: /tests\/perf\//,
      use: { ...devices['Pixel 5'], viewport: { width: 390, height: 844 } }
    },
    {
      name: 'perf',
      testMatch: /tests\/perf\//,
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } }
    }
  ],
  webServer: {
    command: [
      'rm -rf .tmp/e2e-data && mkdir -p .tmp/e2e-data',
      'HQ_WEB_DIR=build HQ_ALLOW_SEED=1',
      '"${HQ_BIN:-../target/debug/rizal-agent-hq}" serve --data .tmp/e2e-data --seed tests/fixtures/e2e.json --allow-seed --port 8901'
    ].join(' && '),
    url: 'http://127.0.0.1:8901',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000
  },
  expect: { toHaveScreenshot: { maxDiffPixelRatio: 0.001, animations: 'disabled' } }
});
