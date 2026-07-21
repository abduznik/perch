// Octopus frame definitions — all states MUST be exactly 7 lines, same width.
// Run: node src/mascot-frames.js

const W = 16;

function p(s) {
  const pad = W - s.length;
  return pad > 0 ? s + ' '.repeat(pad) : s.slice(0, W);
}

const FRAMES = {
  // ── Idle: green octopus, gentle blink ──────────────────────────────
  Idle: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  o   o  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
  Idle2: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  o   O  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
  Idle3: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  O   o  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],

  // ── Working: yellow, tentacles wiggle + hop ─────────────────────────
  Working: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  *   *  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p(" \\| | | | |/   "),
    p("  )( (   ) )(  "),
  ],
  Working2: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  *   *  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
  Working3: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  *   *  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p(" /| | | | |\\   "),
    p("()(   ( )   )()"),
  ],
  Working4: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  *   *  \\   "),
    p(" |     __    |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],

  // ── Done: bright green, happy ^ ^, sparkles ────────────────────────
  Done: [
    p(" *   _____   *  "),
    p("   .'     '.    "),
    p("  /  ^   ^  \\   "),
    p(" |    ~~     |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
  Done2: [
    p("     _____      "),
    p(" * .'     '. *  "),
    p("  /  ^   ^  \\   "),
    p(" |    ~~     |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
  Done3: [
    p("  *  _____  *   "),
    p("   .'     '.    "),
    p("  /  ^   ^  \\   "),
    p(" |    ~~     |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],

  // ── Error: red, concerned o o, beak "!" ────────────────────────────
  Error: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  o   o  \\   "),
    p(" |     !     |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
  Error2: [
    p("     _____      "),
    p("   .'     '.    "),
    p("  /  o   o  \\   "),
    p(" |     !     |  "),
    p("  \\________/   "),
    p("  /| | | | \\   "),
    p(" (  (     )  )  "),
  ],
};

const STATE_FRAMES = {
  Idle:    [FRAMES.Idle, FRAMES.Idle2, FRAMES.Idle3],
  Working: [FRAMES.Working, FRAMES.Working2, FRAMES.Working3, FRAMES.Working4],
  Done:    [FRAMES.Done, FRAMES.Done2, FRAMES.Done3],
  Error:   [FRAMES.Error, FRAMES.Error2],
};

function stripSpans(s) {
  return s.replace(/<[^>]+>/g, '');
}

function validateFrames() {
  const errors = [];
  for (const [state, frames] of Object.entries(STATE_FRAMES)) {
    for (let fi = 0; fi < frames.length; fi++) {
      const frame = frames[fi];
      if (frame.length !== 7) {
        errors.push(`${state}[${fi}]: ${frame.length} lines, expected 7`);
      }
      const lengths = frame.map(l => stripSpans(l).length);
      const unique = new Set(lengths);
      if (unique.size !== 1) {
        errors.push(`${state}[${fi}]: line lengths: ${JSON.stringify(lengths)}`);
      }
    }
  }
  return errors;
}

if (typeof window === 'undefined') {
  const errors = validateFrames();
  if (errors.length > 0) {
    console.error('FRAME VALIDATION FAILED:');
    errors.forEach(e => console.error('  ' + e));
    process.exit(1);
  } else {
    console.log('All frames valid: 7 lines, consistent width');
    for (const [state, frames] of Object.entries(STATE_FRAMES)) {
      console.log(`\n--- ${state} (${frames.length} frames) ---`);
      for (let fi = 0; fi < frames.length; fi++) {
        console.log(`Frame ${fi}:`);
        frames[fi].forEach(line => console.log('  |' + stripSpans(line) + '|'));
      }
    }
  }
}
