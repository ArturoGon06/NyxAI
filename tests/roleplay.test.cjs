const test = require("node:test");
const assert = require("node:assert/strict");
const { parseRoleplaySegments } = require("../public/roleplay.js");

test("keeps raw text intact while recognizing mixed roleplay segments", () => {
  const raw = '*She looks away.*\n\n"I did not expect you."\nThe room falls silent.';
  const segments = parseRoleplaySegments(raw);

  assert.deepEqual(segments.map(({ kind }) => kind), ["action", "plain", "dialogue", "plain"]);
  assert.equal(segments.map(({ raw: source }) => source).join(""), raw);
  assert.equal(segments[0].text, "She looks away.");
  assert.equal(segments[2].text, '"I did not expect you."');
});

test("recognizes bold actions and curly quoted dialogue", () => {
  const segments = parseRoleplaySegments("**She folds her arms.** “Welcome back.”");

  assert.deepEqual(segments.map(({ kind }) => kind), ["action", "plain", "dialogue"]);
  assert.equal(segments[0].text, "She folds her arms.");
  assert.equal(segments[2].text, "“Welcome back.”");
});

test("recognizes short and long action conventions without changing raw text", () => {
  for (const [raw, kind, text] of [
    ["**word**", "action", "word"],
    ["**two words**", "action", "two words"],
    ["**this is longer action text**", "action", "this is longer action text"],
    ["*word*", "action", "word"],
    ['"dialogue"', "dialogue", '"dialogue"'],
  ]) {
    const segments = parseRoleplaySegments(raw);
    assert.deepEqual(segments.map(({ kind: segmentKind }) => segmentKind), [kind]);
    assert.equal(segments[0].text, text);
    assert.equal(segments.map(({ raw: source }) => source).join(""), raw);
  }

  const mixed = parseRoleplaySegments('**word** "dialogue" plain *action*');
  assert.deepEqual(mixed.map(({ kind }) => kind), ["action", "plain", "dialogue", "plain", "action"]);
  assert.equal(mixed.map(({ raw }) => raw).join(""), '**word** "dialogue" plain *action*');
});

test("leaves ambiguous and malformed formatting as visible plain text", () => {
  for (const raw of ["Unclosed *action", 'Unclosed "dialogue', "*Nested ** formatting", "**Unclosed bold*"]) {
    const segments = parseRoleplaySegments(raw);
    assert.equal(segments.map(({ raw: source }) => source).join(""), raw);
    assert.ok(segments.some(({ kind }) => kind === "plain"));
  }
});
