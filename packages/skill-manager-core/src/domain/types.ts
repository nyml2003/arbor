declare const skillIdBrand: unique symbol;
declare const skillNameBrand: unique symbol;
declare const skillVersionBrand: unique symbol;
declare const contentHashBrand: unique symbol;

export type SkillId = string & Readonly<{ [skillIdBrand]: "SkillId" }>;
export type SkillName = string & Readonly<{ [skillNameBrand]: "SkillName" }>;
export type SkillVersion = string & Readonly<{ [skillVersionBrand]: "SkillVersion" }>;
export type ContentHash = string & Readonly<{ [contentHashBrand]: "ContentHash" }>;

export type SkillFormat = "agent-skill";
export type PackageMetadataSource = "source" | "generated";

export type PathSourceSpec = Readonly<{
  type: "path";
  path: string;
}>;

export type GitSourceSpec = Readonly<{
  type: "git";
  repo: string;
  path: string;
  ref: string;
}>;

export type TarballSourceSpec = Readonly<{
  type: "tarball";
  url: string;
  path: string;
  integrity: string;
}>;

export type NpmSourceSpec = Readonly<{
  type: "npm";
  package: string;
  version: SkillVersion;
  path: string;
}>;

export type SourceSpec = PathSourceSpec | GitSourceSpec | TarballSourceSpec | NpmSourceSpec;

export type SkillDeclaration = Readonly<{
  id: SkillId;
  version: SkillVersion;
  source: SourceSpec;
}>;

export type ArborSkillsManifest = Readonly<{
  schema: "arbor.skills/v1";
  targetDir: string;
  skills: ReadonlyArray<SkillDeclaration>;
}>;

export type SkillMarkdown = Readonly<{
  filePath: string;
  name: SkillName;
  description: string;
  body: string;
  frontMatter: Readonly<Record<string, string>>;
}>;

export type SkillPackageManifest = Readonly<{
  schema: "arbor.skill-package/v1";
  id: SkillId;
  name: SkillName;
  version: SkillVersion;
  format: SkillFormat;
  files: ReadonlyArray<string>;
}>;

export type SourceSkill = Readonly<{
  sourceDir: string;
  skillFilePath: string;
  packageFilePath: string;
  packageFileExists: boolean;
}>;

export type SkillPackage = Readonly<{
  sourceDir: string;
  skill: SkillMarkdown;
  manifest: SkillPackageManifest;
  packageMetadataSource: PackageMetadataSource;
}>;

export type InstallMode = "copy";

export type LockedPathSource = Readonly<{
  type: "path";
  path: string;
  resolvedPath: string;
}>;

export type LockedSource = LockedPathSource;

export type SkillLockEntry = Readonly<{
  name: SkillName;
  version: SkillVersion;
  packageMetadataSource: PackageMetadataSource;
  source: LockedSource;
  contentHash: ContentHash;
  install: Readonly<{
    targetDir: string;
    path: string;
    mode: InstallMode;
  }>;
}>;

export type ArborSkillsLock = Readonly<{
  schema: "arbor.skills-lock/v1";
  generatedAt: string;
  skills: Readonly<Record<string, SkillLockEntry>>;
}>;

export type InstallPlanItem = Readonly<{
  declaration: SkillDeclaration;
  skillPackage: SkillPackage;
  sourceDir: string;
  targetPath: string;
}>;

export type InstallPlan = Readonly<{
  manifest: ArborSkillsManifest;
  manifestPath: string;
  manifestDir: string;
  targetDir: string;
  items: ReadonlyArray<InstallPlanItem>;
}>;

export type InstalledSkillReport = Readonly<{
  id: SkillId;
  name: SkillName;
  version: SkillVersion;
  packageMetadataSource: PackageMetadataSource;
  targetPath: string;
  contentHash: ContentHash;
  action: "planned" | "installed";
}>;

export type InstallReport = Readonly<{
  dryRun: boolean;
  installed: ReadonlyArray<InstalledSkillReport>;
  lockPath: string;
}>;

export type PruneAction =
  | Readonly<{ type: "remove-lock-entry"; id: SkillId }>
  | Readonly<{ type: "remove-empty-dir"; path: string }>
  | Readonly<{ type: "remove-managed-dir"; id: SkillId; path: string }>;

export type PruneReport = Readonly<{
  dryRun: boolean;
  actions: ReadonlyArray<PruneAction>;
  reports: ReadonlyArray<string>;
  lockPath: string;
}>;

export type LoadManifestInput = Readonly<{
  manifestPath: string;
  cwd: string;
}>;

export type LintManifestInput = Readonly<{
  manifestPath: string;
  cwd: string;
}>;

export type CreateInstallPlanInput = Readonly<{
  manifestPath: string;
  cwd: string;
}>;

export type InstallSkillsInput = Readonly<{
  manifestPath: string;
  cwd: string;
  dryRun: boolean;
  pruneLock: boolean;
  nowIso: string;
}>;

export type PruneSkillsInput = Readonly<{
  manifestPath: string;
  cwd: string;
  dryRun: boolean;
  nowIso: string;
}>;

export type ReadLockInput = Readonly<{
  manifestPath: string;
  cwd: string;
}>;
