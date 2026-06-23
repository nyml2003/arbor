import assert from "node:assert/strict";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { runCli } from "../src/cli.js";

type FetchCall = Readonly<{
  url: string;
  init: RequestInit;
}>;

test("--version prints package version", async () => {
  const result = await runCli(["--version"]);

  assert.equal(result.exitCode, 0);
  assert.equal(result.stdout, "0.1.0\n");
  assert.equal(result.stderr, "");
});

test("--help prints usage", async () => {
  const result = await runCli(["--help"]);

  assert.equal(result.exitCode, 0);
  assert.match(result.stdout, /Usage:/);
  assert.match(result.stdout, /DEEPSEEK_API_KEY/);
});

test("no prompt enters multi-turn chat", async () => {
  let calls = 0;
  const result = await runCli([], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    readLine: createLineReader(["/exit"]),
    fetch: async () => {
      calls += 1;
      return jsonResponse(200, { choices: [] });
    },
  });

  assert.equal(result.exitCode, 0);
  assert.match(result.stdout, /Aster chat/);
  assert.equal(result.stderr, "");
  assert.equal(calls, 0);
});

test("missing api key fails before calling DeepSeek", async () => {
  let called = false;
  const result = await runCli(["hello"], {
    env: {},
    fetch: async () => {
      called = true;
      return jsonResponse(200, { choices: [] });
    },
  });

  assert.equal(result.exitCode, 1);
  assert.equal(result.stderr, "Missing DEEPSEEK_API_KEY.\n");
  assert.equal(called, false);
});

test("empty chat input does not call DeepSeek", async () => {
  let calls = 0;
  const result = await runCli([], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    readLine: createLineReader(["", "   ", "/quit"]),
    fetch: async () => {
      calls += 1;
      return jsonResponse(200, { choices: [] });
    },
  });

  assert.equal(result.exitCode, 0);
  assert.equal(result.stderr, "");
  assert.equal(calls, 0);
});

test("prompt, system, and model are mapped into the request body", async () => {
  const calls: FetchCall[] = [];
  const result = await runCli(["--system", "Be precise.", "--model", "deepseek-v4-pro", "explain", "this"], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    baseUrl: "https://example.test",
    fetch: async (input, init) => {
      calls.push({
        url: String(input),
        init: init ?? {},
      });
      return streamResponse(["done"]);
    },
  });

  assert.equal(result.exitCode, 0);
  assert.equal(result.stdout, "done\n\n--- rendered markdown ---\ndone\n");
  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.url, "https://example.test/chat/completions");
  assert.equal(calls[0]?.init.method, "POST");
  assert.deepEqual(calls[0]?.init.headers, {
    "Content-Type": "application/json",
    Authorization: "Bearer test-key",
  });

  const body = JSON.parse(String(calls[0]?.init.body)) as Readonly<Record<string, unknown>>;
  assert.equal(body["model"], "deepseek-v4-pro");
  assert.equal(body["stream"], true);
  assert.deepEqual(body["thinking"], { type: "disabled" });
  assert.deepEqual(body["messages"], [
    { role: "system", content: "Be precise." },
    { role: "user", content: "explain this" },
  ]);
});

test("defaults to deepseek-v4-flash", async () => {
  let body: Readonly<Record<string, unknown>> | null = null;
  const result = await runCli(["hello"], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    fetch: async (_input, init) => {
      body = JSON.parse(String(init?.body)) as Readonly<Record<string, unknown>>;
      return streamResponse(["world"]);
    },
  });

  assert.equal(result.exitCode, 0);
  assert.equal(body?.["model"], "deepseek-v4-flash");
  assert.equal(body?.["stream"], true);
});

test("--json prints raw response", async () => {
  const result = await runCli(["--json", "hello"], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    fetch: async () => jsonResponse(200, {
      id: "chatcmpl-test",
      choices: [{ message: { content: "world" } }],
    }),
  });

  assert.equal(result.exitCode, 0);
  assert.match(result.stdout, /"id": "chatcmpl-test"/);
  assert.match(result.stdout, /"content": "world"/);
});

test("non-2xx DeepSeek response is readable", async () => {
  const result = await runCli(["hello"], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    fetch: async () => jsonResponse(401, {
      error: { message: "Invalid API key" },
    }),
  });

  assert.equal(result.exitCode, 1);
  assert.equal(result.stderr, "DeepSeek API error 401: Invalid API key\n");
});

test("empty assistant content is an error", async () => {
  const result = await runCli(["hello"], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    fetch: async () => streamResponse([]),
  });

  assert.equal(result.exitCode, 1);
  assert.equal(result.stderr, "DeepSeek stream did not include assistant content.\n");
});

test("multi-turn chat sends previous assistant reply", async () => {
  const calls: FetchCall[] = [];
  const result = await runCli([], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    readLine: createLineReader(["hello", "again", "/exit"]),
    fetch: async (input, init) => {
      calls.push({
        url: String(input),
        init: init ?? {},
      });
      return streamResponse([calls.length === 1 ? "first answer" : "second answer"]);
    },
  });

  assert.equal(result.exitCode, 0);
  assert.match(result.stdout, /first answer/);
  assert.match(result.stdout, /second answer/);
  assert.equal(calls.length, 2);

  const secondBody = JSON.parse(String(calls[1]?.init.body)) as Readonly<Record<string, unknown>>;
  assert.deepEqual(secondBody["messages"], [
    { role: "user", content: "hello" },
    { role: "assistant", content: "first answer" },
    { role: "user", content: "again" },
  ]);
});

test("system prompt appears once in multi-turn chat history", async () => {
  const calls: FetchCall[] = [];
  const result = await runCli(["--system", "Be short."], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    readLine: createLineReader(["hello", "again", "/exit"]),
    fetch: async (input, init) => {
      calls.push({
        url: String(input),
        init: init ?? {},
      });
      return streamResponse([calls.length === 1 ? "one" : "two"]);
    },
  });

  assert.equal(result.exitCode, 0);
  assert.equal(calls.length, 2);

  const secondBody = JSON.parse(String(calls[1]?.init.body)) as Readonly<Record<string, unknown>>;
  assert.deepEqual(secondBody["messages"], [
    { role: "system", content: "Be short." },
    { role: "user", content: "hello" },
    { role: "assistant", content: "one" },
    { role: "user", content: "again" },
  ]);
});

test("failed multi-turn request does not enter history", async () => {
  const calls: FetchCall[] = [];
  const result = await runCli([], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    readLine: createLineReader(["bad", "good", "/exit"]),
    fetch: async (input, init) => {
      calls.push({
        url: String(input),
        init: init ?? {},
      });
      if (calls.length === 1) {
        return jsonResponse(500, {
          error: { message: "temporary failure" },
        });
      }
      return streamResponse(["recovered"]);
    },
  });

  assert.equal(result.exitCode, 0);
  assert.match(result.stderr, /DeepSeek API error 500: temporary failure/);
  assert.match(result.stdout, /recovered/);
  assert.equal(calls.length, 2);

  const secondBody = JSON.parse(String(calls[1]?.init.body)) as Readonly<Record<string, unknown>>;
  assert.deepEqual(secondBody["messages"], [
    { role: "user", content: "good" },
  ]);
});

test("streaming chunks are printed before rendered markdown", async () => {
  const result = await runCli(["hello"], {
    env: { DEEPSEEK_API_KEY: "test-key" },
    fetch: async () => streamResponse(["# Title\n", "- **bold** and `code`"]),
  });

  assert.equal(result.exitCode, 0);
  assert.match(result.stdout, /^# Title\n- \*\*bold\*\* and `code`/);
  assert.match(result.stdout, /--- rendered markdown ---/);
  assert.match(result.stdout, /\x1b\[1m\x1b\[36mTitle\x1b\[0m/);
  assert.match(result.stdout, /\x1b\[1mbold\x1b\[0m/);
  assert.match(result.stdout, /\x1b\[7mcode\x1b\[0m/);
});

test("--skill loads a skill from default search dirs", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill(join(workspace, ".codex", "skills", "writer"), {
      name: "writer",
      description: "Use for concise writing.",
      body: "Write plainly.",
    });

    const calls: FetchCall[] = [];
    const result = await runCli(["--skill", "writer", "revise this"], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      fetch: async (input, init) => {
        calls.push({ url: String(input), init: init ?? {} });
        return streamResponse(["done"]);
      },
    });

    assert.equal(result.exitCode, 0);
    const body = JSON.parse(String(calls[0]?.init.body)) as Readonly<Record<string, unknown>>;
    assert.deepEqual(body["messages"], [
      {
        role: "system",
        content: [
          "Active skills:",
          "Use the following local SKILL.md instructions as additional operating guidance for this answer.",
          "",
          "## Skill 1: writer",
          "Description: Use for concise writing.",
          `Source: ${join(workspace, ".codex", "skills", "writer", "SKILL.md")}`,
          "",
          "Write plainly.",
        ].join("\n"),
      },
      { role: "user", content: "revise this" },
    ]);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("--skill loads a skill directory path", async () => {
  const workspace = await createWorkspace();
  try {
    const skillDir = join(workspace, "custom-skill");
    await writeSkill(skillDir, {
      name: "custom-skill",
      description: "Use for custom behavior.",
      body: "Prefer concrete examples.",
    });

    let requestBody: Readonly<Record<string, unknown>> | null = null;
    const result = await runCli(["--skill", "custom-skill", "explain"], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      fetch: async (_input, init) => {
        requestBody = JSON.parse(String(init?.body)) as Readonly<Record<string, unknown>>;
        return streamResponse(["done"]);
      },
    });

    assert.equal(result.exitCode, 0);
    assert.match(JSON.stringify(requestBody), /Prefer concrete examples/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("--skill loads a direct markdown file", async () => {
  const workspace = await createWorkspace();
  try {
    const skillPath = join(workspace, "direct.md");
    await writeFile(
      skillPath,
      "---\nname: direct\ndescription: Use direct file.\n---\n\nDirect body.",
      "utf8",
    );

    let requestBody: Readonly<Record<string, unknown>> | null = null;
    const result = await runCli(["--skill", "direct.md", "go"], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      fetch: async (_input, init) => {
        requestBody = JSON.parse(String(init?.body)) as Readonly<Record<string, unknown>>;
        return streamResponse(["done"]);
      },
    });

    assert.equal(result.exitCode, 0);
    assert.match(JSON.stringify(requestBody), /Direct body/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("multiple --skill values preserve order after --system", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill(join(workspace, ".codex", "skills", "first"), {
      name: "first",
      description: "First skill.",
      body: "First body.",
    });
    await writeSkill(join(workspace, ".codex", "skills", "second"), {
      name: "second",
      description: "Second skill.",
      body: "Second body.",
    });

    let requestBody: Readonly<Record<string, unknown>> | null = null;
    const result = await runCli(["--system", "Base system.", "--skill", "first", "--skill", "second", "go"], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      fetch: async (_input, init) => {
        requestBody = JSON.parse(String(init?.body)) as Readonly<Record<string, unknown>>;
        return streamResponse(["done"]);
      },
    });

    assert.equal(result.exitCode, 0);
    if (requestBody === null) {
      throw new Error("Expected request body.");
    }
    const messages = requestBody["messages"] as unknown;
    assert.equal(Array.isArray(messages), true);
    const messageList = messages as ReadonlyArray<Readonly<Record<string, unknown>>>;
    const systemMessage = String(messageList[0]?.["content"]);
    assert.match(systemMessage, /^Base system\.\n\nActive skills:/);
    assert.ok(systemMessage.indexOf("## Skill 1: first") < systemMessage.indexOf("## Skill 2: second"));
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("--skill-dir adds a search directory", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill(join(workspace, "extra", "helper"), {
      name: "helper",
      description: "Extra helper.",
      body: "Extra body.",
    });

    let requestBody: Readonly<Record<string, unknown>> | null = null;
    const result = await runCli(["--skill-dir", "extra", "--skill", "helper", "go"], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      fetch: async (_input, init) => {
        requestBody = JSON.parse(String(init?.body)) as Readonly<Record<string, unknown>>;
        return streamResponse(["done"]);
      },
    });

    assert.equal(result.exitCode, 0);
    assert.match(JSON.stringify(requestBody), /Extra body/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("REPL /skill loads a skill for later messages", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill(join(workspace, ".codex", "skills", "plain-tech-writing-cn"), {
      name: "plain-tech-writing-cn",
      description: "Use for plain Chinese technical writing.",
      body: "Write directly.",
    });

    const calls: FetchCall[] = [];
    const result = await runCli([], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      readLine: createLineReader(["/skill plain-tech-writing-cn", "这个skill讲了什么", "/exit"]),
      fetch: async (input, init) => {
        calls.push({ url: String(input), init: init ?? {} });
        return streamResponse(["done"]);
      },
    });

    assert.equal(result.exitCode, 0);
    assert.match(result.stdout, /Loaded skill: plain-tech-writing-cn/);
    assert.equal(calls.length, 1);

    const body = JSON.parse(String(calls[0]?.init.body)) as Readonly<Record<string, unknown>>;
    assert.match(JSON.stringify(body["messages"]), /Write directly/);
    assert.match(JSON.stringify(body["messages"]), /这个skill讲了什么/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("REPL /skills lists loaded skills", async () => {
  const workspace = await createWorkspace();
  try {
    await writeSkill(join(workspace, ".codex", "skills", "writer"), {
      name: "writer",
      description: "Use for writing.",
      body: "Write.",
    });

    const result = await runCli([], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      readLine: createLineReader(["/skills", "/skill writer", "/skills", "/exit"]),
      fetch: async () => streamResponse(["done"]),
    });

    assert.equal(result.exitCode, 0);
    assert.match(result.stdout, /No skills loaded/);
    assert.match(result.stdout, /- writer:/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

test("missing --skill returns checked paths", async () => {
  const workspace = await createWorkspace();
  try {
    const result = await runCli(["--skill", "missing", "go"], {
      env: { DEEPSEEK_API_KEY: "test-key" },
      cwd: workspace,
      fetch: async () => streamResponse(["done"]),
    });

    assert.equal(result.exitCode, 1);
    assert.match(result.stderr, /Skill not found: missing/);
    assert.match(result.stderr, /Checked:/);
    assert.match(result.stderr, /\.codex[\\/]skills[\\/]missing/);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json",
    },
  });
}

function streamResponse(parts: ReadonlyArray<string>): Response {
  const lines = parts.map((part) =>
    `data: ${JSON.stringify({ choices: [{ delta: { content: part } }] })}\n\n`,
  );
  lines.push("data: [DONE]\n\n");
  const encoder = new TextEncoder();

  return new Response(new ReadableStream<Uint8Array>({
    start(controller) {
      for (const line of lines) {
        controller.enqueue(encoder.encode(line));
      }
      controller.close();
    },
  }), {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream",
    },
  });
}

function createLineReader(lines: ReadonlyArray<string>): (prompt: string) => Promise<string | null> {
  let index = 0;
  return async () => {
    const line = lines[index];
    index += 1;
    return line ?? null;
  };
}

async function createWorkspace(): Promise<string> {
  const dir = join(tmpdir(), `aster-skill-test-${crypto.randomUUID()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}

async function writeSkill(
  dir: string,
  skill: Readonly<{ name: string; description: string; body: string }>,
): Promise<void> {
  await mkdir(dir, { recursive: true });
  await writeFile(
    join(dir, "SKILL.md"),
    `---\nname: ${skill.name}\ndescription: ${skill.description}\n---\n\n${skill.body}\n`,
    "utf8",
  );
}
