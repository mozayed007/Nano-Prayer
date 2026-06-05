/**
 * Professional Update Checker for NanoPrayer
 * Handles stable and pre-release channels with intelligent defaults
 */

export interface ReleaseInfo {
  version: string;
  tagName: string;
  body: string;
  htmlUrl: string;
  publishedAt: string;
  isPreRelease: boolean;
  assets: Array<{
    name: string;
    size: number;
    downloadUrl: string;
  }>;
}

export interface UpdateCheckResult {
  hasUpdate: boolean;
  currentVersion: string;
  stableRelease: ReleaseInfo | null;
  preRelease: ReleaseInfo | null;
  recommendedRelease: ReleaseInfo | null;
  bothChannelsAvailable: boolean;
  isPreReleaseNewer: boolean;
  error?: string;
}

export type UpdateChannel = 'stable' | 'prerelease' | 'auto';

const GITHUB_API_URL = 'https://api.github.com/repos/mozayed007/Nano-Prayer/releases';
const GITHUB_URL = 'https://github.com/mozayed007/Nano-Prayer/releases';

/**
 * Parse version string into comparable parts
 * Supports: 1.2.3, v1.2.3, 1.2.3-beta, 1.2.3-alpha.1, etc.
 */
export function parseVersion(version: string): {
  major: number;
  minor: number;
  patch: number;
  prerelease: string | null;
  prereleaseNum: number;
} {
  const clean = version.replace(/^v/, '').trim();
  const [versionPart, prereleasePart] = clean.split(/[-+]/);
  const parts = versionPart.split('.').map((n) => parseInt(n, 10) || 0);

  // Parse prerelease number (e.g., "beta.2" -> 2)
  let prereleaseNum = 0;
  if (prereleasePart) {
    const match = prereleasePart.match(/\.(\d+)$/);
    if (match) {
      prereleaseNum = parseInt(match[1], 10) || 0;
    }
  }

  return {
    major: parts[0] || 0,
    minor: parts[1] || 0,
    patch: parts[2] || 0,
    prerelease: prereleasePart || null,
    prereleaseNum,
  };
}

/**
 * Compare two versions
 * Returns: -1 if v1 < v2, 0 if equal, 1 if v1 > v2
 */
export function compareVersions(v1: string, v2: string): number {
  const p1 = parseVersion(v1);
  const p2 = parseVersion(v2);

  // Compare major.minor.patch
  if (p1.major !== p2.major) return p1.major > p2.major ? 1 : -1;
  if (p1.minor !== p2.minor) return p1.minor > p2.minor ? 1 : -1;
  if (p1.patch !== p2.patch) return p1.patch > p2.patch ? 1 : -1;

  // Handle prerelease comparison
  const v1HasPre = p1.prerelease !== null;
  const v2HasPre = p2.prerelease !== null;

  // 1.2.3 > 1.2.3-beta (stable is newer than prerelease of same version)
  if (!v1HasPre && v2HasPre) return 1;
  if (v1HasPre && !v2HasPre) return -1;

  // Both have prerelease - compare them
  if (v1HasPre && v2HasPre) {
    // Simple string comparison for prerelease type (beta > alpha)
    if (p1.prerelease !== p2.prerelease) {
      return p1.prerelease > p2.prerelease ? 1 : -1;
    }
    // Same prerelease type - compare number
    if (p1.prereleaseNum !== p2.prereleaseNum) {
      return p1.prereleaseNum > p2.prereleaseNum ? 1 : -1;
    }
  }

  return 0;
}

/**
 * Check if a version is newer than current
 */
export function isNewerVersion(current: string, latest: string): boolean {
  return compareVersions(current, latest) < 0;
}

/**
 * Format bytes to human readable
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

/**
 * Fetch releases from GitHub API
 * Handles rate limiting, network errors, and malformed responses
 */
export async function fetchReleases(): Promise<{
  stable: ReleaseInfo | null;
  prerelease: ReleaseInfo | null;
  error?: string;
}> {
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 10000); // 10s timeout

    const response = await fetch(GITHUB_API_URL, {
      headers: {
        Accept: 'application/vnd.github.v3+json',
        // Add a user-agent to avoid rate limiting
        'User-Agent': 'NanoPrayer-Updater',
      },
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    if (response.status === 403) {
      // Rate limited - try to parse the reset time
      const resetTime = response.headers.get('X-RateLimit-Reset');
      if (resetTime) {
        const resetDate = new Date(parseInt(resetTime) * 1000);
        return {
          stable: null,
          prerelease: null,
          error: `GitHub API rate limited. Try again after ${resetDate.toLocaleTimeString()}`,
        };
      }
      return {
        stable: null,
        prerelease: null,
        error: 'GitHub API rate limited. Please try again later.',
      };
    }

    if (response.status === 404) {
      return {
        stable: null,
        prerelease: null,
        error: 'No releases found.',
      };
    }

    if (!response.ok) {
      throw new Error(`GitHub API returned ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();

    if (!Array.isArray(data) || data.length === 0) {
      return {
        stable: null,
        prerelease: null,
        error: 'No releases available.',
      };
    }

    // Find latest stable and pre-release
    let stable: ReleaseInfo | null = null;
    let prerelease: ReleaseInfo | null = null;

    for (const release of data) {
      const info: ReleaseInfo = {
        version: release.tag_name.replace(/^v/, ''),
        tagName: release.tag_name,
        body: release.body || 'No release notes available.',
        htmlUrl: release.html_url,
        publishedAt: release.published_at,
        isPreRelease: release.prerelease === true,
        assets: (release.assets || []).map((a: any) => ({
          name: a.name,
          size: a.size,
          downloadUrl: a.browser_download_url,
        })),
      };

      if (info.isPreRelease) {
        // Keep the newest prerelease
        if (!prerelease || isNewerVersion(prerelease.version, info.version)) {
          prerelease = info;
        }
      } else {
        // Keep the newest stable
        if (!stable || isNewerVersion(stable.version, info.version)) {
          stable = info;
        }
      }
    }

    return { stable, prerelease };
  } catch (error) {
    if (error instanceof Error) {
      if (error.name === 'AbortError') {
        return {
          stable: null,
          prerelease: null,
          error: 'Request timed out. Please check your connection.',
        };
      }
      return {
        stable: null,
        prerelease: null,
        error: error.message,
      };
    }
    return {
      stable: null,
      prerelease: null,
      error: 'Unknown error occurred',
    };
  }
}

/**
 * Check for updates with intelligent channel selection
 *
 * Logic:
 * - If only stable releases exist -> recommend stable
 * - If only pre-releases exist -> recommend pre-release (default behavior)
 * - If both exist:
 *   - If pre-release is newer than stable -> recommend pre-release but show both options
 *   - If stable is newer or equal -> recommend stable
 *
 * @param currentVersion Current app version
 * @param preferredChannel User's preferred channel ('stable', 'prerelease', or 'auto')
 * @returns Update check result with recommendation
 */
export async function checkForAppUpdates(
  currentVersion: string,
  preferredChannel: UpdateChannel = 'auto'
): Promise<UpdateCheckResult> {
  const { stable, prerelease, error } = await fetchReleases();

  if (error) {
    return {
      hasUpdate: false,
      currentVersion,
      stableRelease: stable,
      preRelease: prerelease,
      recommendedRelease: null,
      bothChannelsAvailable: false,
      isPreReleaseNewer: false,
      error,
    };
  }

  const hasStable = stable !== null;
  const hasPreRelease = prerelease !== null;
  const bothChannelsAvailable = hasStable && hasPreRelease;

  // Determine if updates are available for each channel
  const stableUpdateAvailable = stable ? isNewerVersion(currentVersion, stable.version) : false;
  const prereleaseUpdateAvailable = prerelease
    ? isNewerVersion(currentVersion, prerelease.version)
    : false;

  // Determine if pre-release is newer than stable
  let isPreReleaseNewer = false;
  if (stable && prerelease) {
    isPreReleaseNewer = isNewerVersion(stable.version, prerelease.version);
  }

  // Determine recommendation based on channel preference
  let recommendedRelease: ReleaseInfo | null = null;

  if (preferredChannel === 'stable') {
    // User wants stable only
    recommendedRelease = stableUpdateAvailable ? stable : null;
  } else if (preferredChannel === 'prerelease') {
    // User wants pre-release
    recommendedRelease = prereleaseUpdateAvailable ? prerelease : null;
  } else {
    // Auto mode - intelligent selection
    if (bothChannelsAvailable) {
      if (isPreReleaseNewer && prereleaseUpdateAvailable) {
        // Pre-release is newer - recommend it
        recommendedRelease = prerelease;
      } else if (stableUpdateAvailable) {
        // Stable is newer or same - recommend stable
        recommendedRelease = stable;
      } else if (prereleaseUpdateAvailable) {
        // Only prerelease has update
        recommendedRelease = prerelease;
      }
    } else if (hasStable && stableUpdateAvailable) {
      recommendedRelease = stable;
    } else if (hasPreRelease && prereleaseUpdateAvailable) {
      recommendedRelease = prerelease;
    }
  }

  const hasUpdate = recommendedRelease !== null;

  return {
    hasUpdate,
    currentVersion,
    stableRelease: stable,
    preRelease: prerelease,
    recommendedRelease,
    bothChannelsAvailable,
    isPreReleaseNewer,
  };
}

/**
 * Get the download URL for the appropriate asset
 * Returns the best matching asset for the install type
 */
export function getDownloadUrl(
  release: ReleaseInfo,
  isPortable: boolean
): string | null {
  const assets = release.assets;

  // Look for portable or setup based on install type
  const searchTerms = isPortable
    ? ['portable', 'Portable']
    : ['setup', 'Setup', 'installer', 'Installer'];

  // First try to find exact match
  for (const term of searchTerms) {
    const asset = assets.find((a) => a.name.includes(term) && a.name.endsWith('.exe'));
    if (asset) {
      return asset.downloadUrl;
    }
  }

  // Fallback: any .exe file
  const anyExe = assets.find((a) => a.name.endsWith('.exe'));
  if (anyExe) {
    return anyExe.downloadUrl;
  }

  // Final fallback: return the release page URL
  return release.htmlUrl;
}

/**
 * Get release page URL for manual download
 */
export function getReleasePageUrl(version: string): string {
  return `${GITHUB_URL}/tag/v${version}`;
}

/**
 * Format release notes with markdown stripping for display
 */
export function formatReleaseNotes(body: string, maxLength: number = 300): string {
  if (!body) return 'No release notes available.';

  // Remove markdown headers
  let cleaned = body.replace(/#{1,6}\s/g, '');
  // Remove markdown links but keep text
  cleaned = cleaned.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
  // Remove other markdown
  cleaned = cleaned.replace(/[*_`]/g, '');
  // Trim whitespace
  cleaned = cleaned.trim();

  if (cleaned.length > maxLength) {
    cleaned = cleaned.substring(0, maxLength).trim() + '...';
  }

  return cleaned;
}
