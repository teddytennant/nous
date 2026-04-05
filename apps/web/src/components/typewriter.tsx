"use client";

import { useEffect, useRef, useState } from "react";

/**
 * TypeWriter — cycles through phrases with a typing/erasing effect.
 * Pure React + CSS. No external dependencies.
 * Respects prefers-reduced-motion: shows phrases statically with crossfade.
 */

interface TypeWriterProps {
  phrases: string[];
  /** Milliseconds between typing each character */
  typeSpeed?: number;
  /** Milliseconds between erasing each character */
  eraseSpeed?: number;
  /** Pause (ms) after fully typing a phrase before erasing */
  pauseAfterType?: number;
  /** Pause (ms) after fully erasing before typing next */
  pauseAfterErase?: number;
  className?: string;
}

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function TypeWriter({
  phrases,
  typeSpeed = 50,
  eraseSpeed = 30,
  pauseAfterType = 2400,
  pauseAfterErase = 400,
  className,
}: TypeWriterProps) {
  const [text, setText] = useState("");
  const [phraseIndex, setPhraseIndex] = useState(0);
  const [isTyping, setIsTyping] = useState(true);
  const [reducedMotion, setReducedMotion] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Check reduced motion on mount
  useEffect(() => {
    setReducedMotion(prefersReducedMotion());
  }, []);

  // Static mode for reduced motion — crossfade between phrases
  const [staticIndex, setStaticIndex] = useState(0);

  useEffect(() => {
    if (!reducedMotion || phrases.length <= 1) return;
    const interval = setInterval(() => {
      setStaticIndex((i) => (i + 1) % phrases.length);
    }, 3000);
    return () => clearInterval(interval);
  }, [reducedMotion, phrases.length]);

  // Typing animation
  useEffect(() => {
    if (reducedMotion || phrases.length === 0) return;

    const currentPhrase = phrases[phraseIndex];

    if (isTyping) {
      if (text.length < currentPhrase.length) {
        timeoutRef.current = setTimeout(() => {
          setText(currentPhrase.slice(0, text.length + 1));
        }, typeSpeed);
      } else {
        // Fully typed — pause then start erasing
        timeoutRef.current = setTimeout(() => {
          setIsTyping(false);
        }, pauseAfterType);
      }
    } else {
      if (text.length > 0) {
        timeoutRef.current = setTimeout(() => {
          setText(text.slice(0, -1));
        }, eraseSpeed);
      } else {
        // Fully erased — pause then move to next phrase
        timeoutRef.current = setTimeout(() => {
          setPhraseIndex((i) => (i + 1) % phrases.length);
          setIsTyping(true);
        }, pauseAfterErase);
      }
    }

    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [
    text,
    phraseIndex,
    isTyping,
    reducedMotion,
    phrases,
    typeSpeed,
    eraseSpeed,
    pauseAfterType,
    pauseAfterErase,
  ]);

  if (phrases.length === 0) return null;

  // Reduced motion: static crossfade
  if (reducedMotion) {
    return (
      <span className={className} aria-live="polite">
        {phrases[staticIndex]}
      </span>
    );
  }

  return (
    <span className={className} aria-live="polite" aria-label={phrases[phraseIndex]}>
      {text}
      <span className="typewriter-cursor" aria-hidden="true">
        |
      </span>
    </span>
  );
}
