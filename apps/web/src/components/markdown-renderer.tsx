"use client";

import { useState, useCallback, type ComponentPropsWithoutRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Copy, Check } from "lucide-react";
import { cn } from "@/lib/utils";

// ── Code block with copy button ─────────────────────────────────────────

function CodeBlock({
  className,
  children,
}: {
  className?: string;
  children: string;
}) {
  const [copied, setCopied] = useState(false);

  const lang = className?.replace("language-", "") ?? "";

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API may not be available
    }
  }, [children]);

  return (
    <div className="group/code relative my-4 first:mt-0 last:mb-0">
      <div className="flex items-center justify-between px-4 py-2 bg-white/[0.03] border border-white/[0.06] border-b-0 rounded-t-sm">
        <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
          {lang || "code"}
        </span>
        <button
          onClick={handleCopy}
          className="flex items-center gap-1.5 text-[10px] font-mono text-neutral-600 hover:text-[#d4af37] transition-colors duration-150"
          title="Copy code"
        >
          {copied ? (
            <>
              <Check size={11} className="text-[#d4af37]" />
              <span className="text-[#d4af37]">Copied</span>
            </>
          ) : (
            <>
              <Copy size={11} />
              <span className="opacity-0 group-hover/code:opacity-100 transition-opacity duration-150">
                Copy
              </span>
            </>
          )}
        </button>
      </div>
      <pre className="bg-white/[0.02] border border-white/[0.06] border-t-0 rounded-b-sm px-4 py-3 overflow-x-auto">
        <code className={cn("text-[13px] font-mono leading-relaxed text-neutral-200", className)}>
          {children}
        </code>
      </pre>
    </div>
  );
}

// ── Custom components map ───────────────────────────────────────────────

const components = {
  code(props: ComponentPropsWithoutRef<"code">) {
    const { className, children } = props;
    const content = String(children).replace(/\n$/, "");
    const isBlock = className || content.includes("\n");

    if (isBlock) {
      return <CodeBlock className={className}>{content}</CodeBlock>;
    }

    return (
      <code className="text-[13px] font-mono bg-white/[0.06] border border-white/[0.04] px-1.5 py-0.5 rounded-sm text-[#d4af37]">
        {children}
      </code>
    );
  },

  pre(props: ComponentPropsWithoutRef<"pre">) {
    return <>{props.children}</>;
  },

  h1(props: ComponentPropsWithoutRef<"h1">) {
    return (
      <h1 className="text-xl font-light mt-6 mb-3 first:mt-0 text-white">
        {props.children}
      </h1>
    );
  },
  h2(props: ComponentPropsWithoutRef<"h2">) {
    return (
      <h2 className="text-lg font-light mt-5 mb-2.5 first:mt-0 text-white">
        {props.children}
      </h2>
    );
  },
  h3(props: ComponentPropsWithoutRef<"h3">) {
    return (
      <h3 className="text-base font-medium mt-4 mb-2 first:mt-0 text-white">
        {props.children}
      </h3>
    );
  },
  h4(props: ComponentPropsWithoutRef<"h4">) {
    return (
      <h4 className="text-sm font-medium mt-3 mb-1.5 first:mt-0 text-neutral-200">
        {props.children}
      </h4>
    );
  },

  p(props: ComponentPropsWithoutRef<"p">) {
    return (
      <p className="text-sm font-light leading-relaxed text-neutral-100 mb-3 last:mb-0">
        {props.children}
      </p>
    );
  },

  ul(props: ComponentPropsWithoutRef<"ul">) {
    return <ul className="list-none space-y-1.5 mb-3 last:mb-0">{props.children}</ul>;
  },
  ol(props: ComponentPropsWithoutRef<"ol">) {
    return (
      <ol className="list-none space-y-1.5 mb-3 last:mb-0 counter-reset-list">
        {props.children}
      </ol>
    );
  },
  li(props: ComponentPropsWithoutRef<"li">) {
    const isOrdered = (props as Record<string, unknown>).ordered;
    return (
      <li
        className={cn(
          "text-sm font-light leading-relaxed text-neutral-200 pl-5 relative",
          "before:absolute before:left-0 before:text-neutral-600",
          isOrdered
            ? "before:content-[counter(list-item)_'.'] before:font-mono before:text-[11px] counter-increment-list"
            : "before:content-['—'] before:text-[10px]"
        )}
      >
        {props.children}
      </li>
    );
  },

  blockquote(props: ComponentPropsWithoutRef<"blockquote">) {
    return (
      <blockquote className="border-l-2 border-[#d4af37]/30 pl-4 my-3 first:mt-0 last:mb-0">
        <div className="text-neutral-400 italic">{props.children}</div>
      </blockquote>
    );
  },

  table(props: ComponentPropsWithoutRef<"table">) {
    return (
      <div className="overflow-x-auto my-4 first:mt-0 last:mb-0 border border-white/[0.06] rounded-sm">
        <table className="w-full text-sm">{props.children}</table>
      </div>
    );
  },
  thead(props: ComponentPropsWithoutRef<"thead">) {
    return <thead className="bg-white/[0.03]">{props.children}</thead>;
  },
  th(props: ComponentPropsWithoutRef<"th">) {
    return (
      <th className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-500 px-4 py-2.5 border-b border-white/[0.06]">
        {props.children}
      </th>
    );
  },
  td(props: ComponentPropsWithoutRef<"td">) {
    return (
      <td className="text-sm font-light text-neutral-300 px-4 py-2.5 border-b border-white/[0.04]">
        {props.children}
      </td>
    );
  },

  hr() {
    return <hr className="border-white/[0.06] my-6" />;
  },

  a(props: ComponentPropsWithoutRef<"a">) {
    return (
      <a
        href={props.href}
        target="_blank"
        rel="noopener noreferrer"
        className="text-[#d4af37] hover:text-[#e5c348] underline underline-offset-2 decoration-[#d4af37]/30 hover:decoration-[#d4af37]/60 transition-colors duration-150"
      >
        {props.children}
      </a>
    );
  },

  strong(props: ComponentPropsWithoutRef<"strong">) {
    return <strong className="font-medium text-white">{props.children}</strong>;
  },
  em(props: ComponentPropsWithoutRef<"em">) {
    return <em className="italic text-neutral-300">{props.children}</em>;
  },
  del(props: ComponentPropsWithoutRef<"del">) {
    return <del className="line-through text-neutral-600">{props.children}</del>;
  },
};

// ── Main component ──────────────────────────────────────────────────────

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

export function MarkdownRenderer({ content, className }: MarkdownRendererProps) {
  return (
    <div className={cn("markdown-body", className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={components}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
