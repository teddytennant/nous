"use client";

import { useCallback, useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import {
  messaging,
  type ChannelResponse,
  type MessageResponse,
} from "@/lib/api";

const LOCAL_DID = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 60_000) return "now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  return `${Math.floor(diff / 86_400_000)}d`;
}

export default function MessagesPage() {
  const [channels, setChannels] = useState<ChannelResponse[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageResponse[]>([]);
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [newChannelDid, setNewChannelDid] = useState("");

  const fetchChannels = useCallback(async () => {
    try {
      const chs = await messaging.listChannels(LOCAL_DID);
      setChannels(chs);
      setError(null);
    } catch {
      setError("API offline");
    }
  }, []);

  const fetchMessages = useCallback(async (channelId: string) => {
    try {
      const msgs = await messaging.getMessages(channelId, 100);
      setMessages(msgs);
    } catch {
      setMessages([]);
    }
  }, []);

  useEffect(() => {
    fetchChannels();
    const interval = setInterval(fetchChannels, 5000);
    return () => clearInterval(interval);
  }, [fetchChannels]);

  useEffect(() => {
    if (selected) {
      fetchMessages(selected);
      const interval = setInterval(() => fetchMessages(selected), 3000);
      return () => clearInterval(interval);
    }
  }, [selected, fetchMessages]);

  async function createDM() {
    if (!newChannelDid.trim()) return;
    try {
      const ch = await messaging.createChannel({
        creator_did: LOCAL_DID,
        kind: "direct",
        peer_did: newChannelDid.trim(),
      });
      setChannels((prev) => [...prev, ch]);
      setSelected(ch.id);
      setNewChannelDid("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create channel");
    }
  }

  async function sendMessage() {
    if (!input.trim() || !selected) return;
    try {
      const msg = await messaging.sendMessage({
        channel_id: selected,
        sender_did: LOCAL_DID,
        content: input.trim(),
      });
      setMessages((prev) => [...prev, msg]);
      setInput("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to send");
    }
  }

  const selectedChannel = channels.find((c) => c.id === selected);

  return (
    <div className="flex h-screen">
      {/* Channel list */}
      <div className="w-72 border-r border-white/[0.06] flex flex-col">
        <div className="p-6 pb-4">
          <h1 className="text-lg font-extralight tracking-[-0.02em]">
            Messages
          </h1>
          <p className="text-[10px] font-mono text-neutral-600 mt-1 uppercase tracking-wider">
            {error ? (
              <span className="text-red-500">{error}</span>
            ) : (
              "E2E encrypted via Double Ratchet"
            )}
          </p>
        </div>

        {/* New DM input */}
        <div className="px-6 pb-4">
          <div className="flex gap-2">
            <input
              value={newChannelDid}
              onChange={(e) => setNewChannelDid(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createDM()}
              placeholder="did:key:z6Mk..."
              className="flex-1 bg-white/[0.02] text-[10px] font-mono px-3 py-2 outline-none placeholder:text-neutral-700"
            />
            <button
              onClick={createDM}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors duration-150"
            >
              +
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          {channels.length === 0 && (
            <p className="px-6 text-xs text-neutral-700 font-light">
              No conversations yet
            </p>
          )}
          {channels.map((ch) => (
            <button
              key={ch.id}
              onClick={() => setSelected(ch.id)}
              className={cn(
                "w-full text-left px-6 py-4 transition-colors duration-150 border-b border-white/[0.03]",
                selected === ch.id
                  ? "bg-white/[0.02]"
                  : "hover:bg-white/[0.01]"
              )}
            >
              <div className="flex justify-between items-baseline mb-1.5">
                <span className="text-xs font-mono text-neutral-500 truncate max-w-[160px]">
                  {ch.name || ch.members.find((m) => m !== LOCAL_DID) || ch.id}
                </span>
                <span className="text-[10px] text-neutral-700">
                  {ch.kind}
                </span>
              </div>
              <p className="text-[10px] font-mono text-neutral-700">
                {ch.members.length} member{ch.members.length !== 1 ? "s" : ""}
              </p>
            </button>
          ))}
        </div>
      </div>

      {/* Chat view */}
      <div className="flex-1 flex flex-col">
        <div className="px-8 py-5 border-b border-white/[0.06]">
          {selectedChannel ? (
            <p className="text-xs font-mono text-neutral-500">
              {selectedChannel.name ||
                selectedChannel.members
                  .filter((m) => m !== LOCAL_DID)
                  .join(", ") ||
                selectedChannel.id}
            </p>
          ) : (
            <p className="text-xs text-neutral-700 font-light">
              Select a conversation
            </p>
          )}
        </div>

        <div className="flex-1 overflow-y-auto px-8 py-6 space-y-4">
          {!selected && (
            <div className="flex items-center justify-center h-full">
              <p className="text-sm text-neutral-700 font-light">
                Select a conversation or create a new one
              </p>
            </div>
          )}
          {messages.map((msg) => {
            const isSelf = msg.sender === LOCAL_DID;
            return (
              <div
                key={msg.id}
                className={cn("max-w-[70%]", isSelf ? "ml-auto" : "")}
              >
                <div
                  className={cn(
                    "px-4 py-3 text-sm font-light",
                    isSelf
                      ? "bg-white/[0.04] text-white"
                      : "bg-white/[0.02] text-neutral-300"
                  )}
                >
                  {msg.content}
                </div>
                <p
                  className={cn(
                    "text-[10px] text-neutral-700 mt-1",
                    isSelf ? "text-right" : ""
                  )}
                >
                  {timeAgo(msg.timestamp)}
                </p>
              </div>
            );
          })}
        </div>

        {selected && (
          <div className="px-8 py-5 border-t border-white/[0.06]">
            <div className="flex gap-4">
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && sendMessage()}
                placeholder="Type a message..."
                className="flex-1 bg-transparent text-sm font-light outline-none placeholder:text-neutral-700"
              />
              <button
                onClick={sendMessage}
                className="text-xs font-mono uppercase tracking-wider text-neutral-500 hover:text-[#d4af37] transition-colors duration-150"
              >
                Send
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
