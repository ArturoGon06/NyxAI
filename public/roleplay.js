(function attachRoleplayRenderer(globalScope, factory) {
  const api = factory();
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  globalScope.NyxRoleplay = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function createRoleplayRenderer() {
  function segment(kind, raw, text = raw) {
    return { kind, raw, text };
  }

  function appendSegment(segments, next) {
    const previous = segments[segments.length - 1];
    if (previous && previous.kind === next.kind) {
      previous.raw += next.raw;
      previous.text += next.text;
    } else {
      segments.push(next);
    }
  }

  function findSingleActionClose(source, start) {
    for (let index = start; index < source.length; index += 1) {
      if (source[index] === "*" && source[index - 1] !== "*" && source[index + 1] !== "*") return index;
    }
    return -1;
  }

  function findBoldActionClose(source, start) {
    for (let index = start; index < source.length - 1; index += 1) {
      if (source[index] === "*" && source[index + 1] === "*") return index;
    }
    return -1;
  }

  function hasVisibleContent(value) {
    return value.trim().length > 0;
  }

  function dialogueClose(source, start, opening) {
    const closing = opening === "“" ? "”" : opening;
    return source.indexOf(closing, start);
  }

  // This is intentionally a small convention parser, not a Markdown parser.
  // It returns segment metadata only; callers keep and copy the original text.
  function parseRoleplaySegments(source) {
    const segments = [];
    let plain = "";
    let index = 0;

    function flushPlain() {
      if (plain) appendSegment(segments, segment("plain", plain));
      plain = "";
    }

    while (index < source.length) {
      if (source.startsWith("**", index)) {
        const close = findBoldActionClose(source, index + 2);
        const content = close === -1 ? "" : source.slice(index + 2, close);
        if (close !== -1 && hasVisibleContent(content)) {
          flushPlain();
          const raw = source.slice(index, close + 2);
          appendSegment(segments, segment("action", raw, content));
          index = close + 2;
          continue;
        }

        // A malformed bold opening must remain literal. Advancing past both
        // stars prevents its second star from being misread as *single* action
        // syntax on the following pass.
        plain += "**";
        index += 2;
        continue;
      } else if (source[index] === "*") {
        const close = findSingleActionClose(source, index + 1);
        const content = close === -1 ? "" : source.slice(index + 1, close);
        if (close !== -1 && hasVisibleContent(content)) {
          flushPlain();
          const raw = source.slice(index, close + 1);
          appendSegment(segments, segment("action", raw, content));
          index = close + 1;
          continue;
        }
      } else if (source[index] === '"' || source[index] === "“" || source[index] === "”") {
        const close = dialogueClose(source, index + 1, source[index]);
        if (close > index + 1) {
          flushPlain();
          const raw = source.slice(index, close + 1);
          appendSegment(segments, segment("dialogue", raw));
          index = close + 1;
          continue;
        }
      }
      plain += source[index];
      index += 1;
    }
    flushPlain();
    return segments;
  }

  return { parseRoleplaySegments };
});
