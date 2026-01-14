/**
 * Signature and Ink Related Jokes for Loading States
 *
 * Keep users entertained while waiting for API responses.
 * Designed for a geriatric-friendly audience - clean, simple humor.
 */

export interface Joke {
  setup: string;
  punchline: string;
  /** Delay in ms before showing punchline (default: 1500) */
  delay?: number;
}

/** Collection of signature/ink/document related jokes */
export const JOKES: Joke[] = [
  // Ink & Pen jokes
  {
    setup: "What's black, white, and red all over?",
    punchline: "A newspaper! (Though we prefer digital signatures.)",
  },
  {
    setup: "Why did the pen break up with the pencil?",
    punchline: "It found someone more permanent.",
  },
  {
    setup: "What do you call a signature that tells jokes?",
    punchline: "A pun-dit!",
  },
  {
    setup: "Why was the ink feeling blue?",
    punchline: "Because it was running low on self-esteem.",
  },

  // Knock-knock jokes
  {
    setup: "Knock knock... Who's there?... Ink...",
    punchline: "Ink who?... Ink you should sign this document!",
    delay: 2000,
  },
  {
    setup: "Knock knock... Who's there?... Sign...",
    punchline: "Sign who?... Sign here, please!",
    delay: 2000,
  },
  {
    setup: "Knock knock... Who's there?... Document...",
    punchline: "Document who?... Document wait, let's get this signed!",
    delay: 2000,
  },

  // Document & Signature jokes
  {
    setup: "Why did the contract go to therapy?",
    punchline: "It had too many issues.",
  },
  {
    setup: "What did one signature say to the other?",
    punchline: "You're looking sharp today!",
  },
  {
    setup: "Why do signatures make great friends?",
    punchline: "They're always there when you need them.",
  },
  {
    setup: "What's a document's favorite music?",
    punchline: "Heavy metal... because of all the paperclips!",
  },
  {
    setup: "Why was the PDF so calm?",
    punchline: "Because it was well-formatted.",
  },

  // Technology jokes
  {
    setup: "Why did the e-signature cross the road?",
    punchline: "To get to the other side... of the document!",
  },
  {
    setup: "What do you call a lazy signature?",
    punchline: "A sign of the times.",
  },
  {
    setup: "Why are digital signatures so reliable?",
    punchline: "They never lose their pen!",
  },
];

/** Track which jokes have been shown to avoid repeats */
let shownJokeIndices: Set<number> = new Set();

/**
 * Get a random joke, avoiding recently shown ones
 */
export function getRandomJoke(): Joke {
  // Reset if all jokes have been shown
  if (shownJokeIndices.size >= JOKES.length) {
    shownJokeIndices.clear();
  }

  // Find an unshown joke
  let index: number;
  do {
    index = Math.floor(Math.random() * JOKES.length);
  } while (shownJokeIndices.has(index));

  shownJokeIndices.add(index);
  return JOKES[index];
}

/**
 * Reset joke history (e.g., on page refresh)
 */
export function resetJokeHistory(): void {
  shownJokeIndices.clear();
}
