export interface ResumeContact {
  label: string;
  value: string;
  href?: string;
}

export interface ResumeProfile {
  name: string;
  title?: string;
  contacts: ResumeContact[];
}

export interface ResumeEducation {
  school: string;
  tags?: string[];
  degree: string;
  major: string;
  range: [string, string];
  highlights?: string[];
}

export interface ResumeExperience {
  company: string;
  role: string;
  range: [string, string];
  bullets: string[];
}

export interface ResumeProject {
  title: string;
  subtitle?: string;
  tags: string[];
  bullets: string[];
}

export interface ResumeDocument {
  profile: ResumeProfile;
  education: ResumeEducation[];
  skills: string[];
  experiences: ResumeExperience[];
  projects: ResumeProject[];
}
