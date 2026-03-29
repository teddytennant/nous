"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";

interface Conversation {
  id: string;
  peer: string;
  lastMessage: string;
  time: string;
  unread: boolean;
}

interface Message {
  id: string;
  sender: "self" | "peer";
  content: string;
  time: string;
}

const conversations: Conversation[] = [
  {
    id: "1",
    peer: "did:key:z6Mk...q7nV",
    lastMessage: "The protocol is live.",
    time: "3m",
    unread: true,
  },
  {
    id: "2",
    peer: "did:key:z6Mk...8fHj",
    lastMessage: "Governance proposal submitted.",
    time: "1h",
    unread: false,
  },
];

const mockMessages: Message[] = [
  { id: "1", sender: "peer", content: "Have you seen the new consensus module?", time: "14:02" },
  { id: "2", sender: "self", content: "Yes. The Raft implementation looks solid.", time: "14:03" },
  { id: "3", sender: "peer", content: "The protocol is live.", time: "14:05" },
];

export default function MessagesPage() {
  const [selected, setSelected] = useState<string>("1");
  const [input, setInput] = useState("");

  return (
    <div className="flex h-screen">
      {/* Conversation list */}
      <div className="w-72 border-r border-white/[0.06] flex flex-col">
        <div className="p-6 pb-4">
          <h1 className="text-lg font-extralight tracking-[-0.02em]">
            Messages
          </h1>
          <p className="text-[10px] font-mono text-neutral-600 mt-1 uppercase tracking-wider">
            E2E encrypted via Double Ratchet
          </p>
        </div>
        <div className="flex-1 overflow-y-auto">
          {conversations.map((conv) => (
            <button
              key={conv.id}
              onClick={() => setSelected(conv.id)}
              className={cn(
                "w-full text-left px-6 py-4 transition-colors duration-150 border-b border-white/[0.03]",
                selected === conv.id
                  ? "bg-white/[0.02]"
                  : "hover:bg-white/[0.01]"
              )}
            >
              <div className="flex justify-between items-baseline mb-1.5">
                <span className="text-xs font-mono text-neutral-500 truncate max-w-[160px]">
                  {conv.peer}
                </span>
                <span className="text-[10px] text-neutral-700">{conv.time}</span>
              </div>
              <p
                className={cn(
                  "text-xs font-light truncate",
                  conv.unread ? "text-white" : "text-neutral-600"
                )}
              >
                {conv.lastMessage}
              </p>
              {conv.unread && (
                <div className="w-1.5 h-1.5 rounded-full bg-[#d4af37] mt-2" />
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Chat view */}
      <div className="flex-1 flex flex-col">
        <div className="px-8 py-5 border-b border-white/[0.06]">
          <p className="text-xs font-mono text-neutral-500">
            {conversations.find((c) => c.id === selected)?.peer}
          </p>
        </div>

        <div className="flex-1 overflow-y-auto px-8 py-6 space-y-4">
          {mockMessages.map((msg) => (
            <div
              key={msg.id}
              className={cn(
                "max-w-[70%]",
                msg.sender === "self" ? "ml-auto" : ""
              )}
            >
              <div
                className={cn(
                  "px-4 py-3 text-sm font-light",
                  msg.sender === "self"
                    ? "bg-white/[0.04] text-white"
                    : "bg-white/[0.02] text-neutral-300"
                )}
              >
                {msg.content}
              </div>
              <p
                className={cn(
                  "text-[10px] text-neutral-700 mt-1",
                  msg.sender === "self" ? "text-right" : ""
                )}
              >
                {msg.time}
              </p>
            </div>
          ))}
        </div>

        <div className="px-8 py-5 border-t border-white/[0.06]">
          <div className="flex gap-4">
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Type a message..."
              className="flex-1 bg-transparent text-sm font-light outline-none placeholder:text-neutral-700"
            />
            <button className="text-xs font-mono uppercase tracking-wider text-neutral-500 hover:text-[#d4af37] transition-colors duration-150">
              Send
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
