"use client";

import { useCallback, useEffect, useRef, useState, startTransition } from "react";
import { cn } from "@/lib/utils";
import { messaging, type ChannelResponse, type MessageResponse } from "@/lib/api";
import { useRealtime } from "@/lib/use-realtime";

type CreateMode = "dm" | "group" | null;

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 60_000) return "now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  return `${Math.floor(diff / 86_400_000)}d`;
}

function truncateDid(did: string): string {
  if (did.length > 30) return `${did.slice(0, 16)}...${did.slice(-6)}`;
  return did;
}

function channelDisplayName(ch: ChannelResponse, localDid: string): string {
  if (ch.name) return ch.name;
  const others = ch.members.filter((m) => m !== localDid);
  if (others.length > 0) return others.map(truncateDid).join(", ");
  return truncateDid(ch.id);
}

export default function MessagesPage() {
  const [channels, setChannels] = useState<ChannelResponse[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageResponse[]>([]);
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [createMode, setCreateMode] = useState<CreateMode>(null);
  const [newDmDid, setNewDmDid] = useState("");
  const [newGroupName, setNewGroupName] = useState("");
  const [newGroupMembers, setNewGroupMembers] = useState("");
  const [replyTo, setReplyTo] = useState<MessageResponse | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const userDid = typeof window !== "undefined" ? localStorage.getItem("nous_did") || "" : "";

  const fetchChannels = useCallback(async () => {
    if (!userDid) return;
    try {
      const chs = await messaging.listChannels(userDid);
      setChannels(chs);
      setError(null);
    } catch {
      setError("API offline");
    }
  }, [userDid]);

  const fetchMessages = useCallback(async (channelId: string) => {
    try {
      const msgs = await messaging.getMessages(channelId, 100);
      setMessages(msgs);
    } catch {
      setMessages([]);
    }
  }, []);

  useEffect(() => {
    startTransition(() => {
      fetchChannels();
    });
    const interval = setInterval(fetchChannels, 5000);
    return () => clearInterval(interval);
  }, [fetchChannels]);

  useEffect(() => {
    if (selected) {
      startTransition(() => {
        fetchMessages(selected);
      });
      const interval = setInterval(() => fetchMessages(selected), 3000);
      return () => clearInterval(interval);
    }
  }, [selected, fetchMessages]);

  // Live message updates via SSE
  useRealtime("new_message", (data) => {
    if (selected && data.channel_id === selected) {
      setMessages((prev) => [
        ...prev,
        {
          id: `live-${Date.now()}`,
          channel_id: data.channel_id,
          sender: data.sender,
          content: data.content,
          reply_to: null,
          timestamp: new Date().toISOString(),
        },
      ]);
    }
  });

  // Auto-scroll
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function createDM() {
    if (!newDmDid.trim() || !userDid) return;
    try {
      const ch = await messaging.createChannel({
        creator_did: userDid,
        kind: "direct",
        peer_did: newDmDid.trim(),
      });
      setChannels((prev) => [...prev, ch]);
      setSelected(ch.id);
      setNewDmDid("");
      setCreateMode(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create channel");
    }
  }

  async function createGroup() {
    if (!newGroupName.trim() || !userDid) return;
    const members = newGroupMembers.split("\n").map((s) => s.trim()).filter(Boolean);
    try {
      const ch = await messaging.createChannel({
        creator_did: userDid,
        kind: "group",
        name: newGroupName.trim(),
        members,
      });
      setChannels((prev) => [...prev, ch]);
      setSelected(ch.id);
      setNewGroupName("");
      setNewGroupMembers("");
      setCreateMode(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create group");
    }
  }

  async function sendMessage() {
    if (!input.trim() || !selected || !userDid) return;
    try {
      const msg = await messaging.sendMessage({
        channel_id: selected,
        sender_did: userDid,
        content: input.trim(),
        reply_to: replyTo?.id,
      });
      setMessages((prev) => [...prev, msg]);
      setInput("");
      setReplyTo(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to send");
    }
  }

  async function deleteMessage(messageId: string) {
    try {
      await messaging.deleteMessage(messageId);
      setMessages((prev) => prev.filter((m) => m.id !== messageId));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to delete");
    }
  }

  const selectedChannel = channels.find((c) => c.id === selected);

  return (
    <div className="flex h-screen">
      {/* Channel list */}
      <div className="w-72 border-r border-white/[0.06] flex flex-col">
        <div className="p-6 pb-4">
          <h1 className="text-lg font-extralight tracking-[-0.02em]">Messages</h1>
          <p className="text-[10px] font-mono text-neutral-600 mt-1 uppercase tracking-wider">
            {error ? <span className="text-red-500">{error}</span> : "E2E encrypted"}
          </p>
        </div>

        {/* Create channel */}
        <div className="px-6 pb-4">
          {createMode === null ? (
            <div className="flex gap-2">
              <button
                onClick={() => setCreateMode("dm")}
                className="flex-1 text-[10px] font-mono uppercase tracking-wider py-2 border border-white/[0.06] text-neutral-600 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all"
              >
                New DM
              </button>
              <button
                onClick={() => setCreateMode("group")}
                className="flex-1 text-[10px] font-mono uppercase tracking-wider py-2 border border-white/[0.06] text-neutral-600 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all"
              >
                New Group
              </button>
            </div>
          ) : createMode === "dm" ? (
            <div className="space-y-2">
              <input
                value={newDmDid}
                onChange={(e) => setNewDmDid(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && createDM()}
                placeholder="did:key:z6Mk..."
                className="w-full bg-white/[0.02] text-[10px] font-mono px-3 py-2 outline-none placeholder:text-neutral-700"
                autoFocus
              />
              <div className="flex gap-2">
                <button onClick={createDM} className="flex-1 text-[10px] font-mono uppercase tracking-wider py-1.5 border border-[#d4af37]/30 text-[#d4af37]">
                  Create
                </button>
                <button onClick={() => setCreateMode(null)} className="text-[10px] font-mono text-neutral-600 hover:text-white px-2">
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              <input
                value={newGroupName}
                onChange={(e) => setNewGroupName(e.target.value)}
                placeholder="Group name"
                className="w-full bg-white/[0.02] text-[10px] font-mono px-3 py-2 outline-none placeholder:text-neutral-700"
                autoFocus
              />
              <textarea
                value={newGroupMembers}
                onChange={(e) => setNewGroupMembers(e.target.value)}
                placeholder="Member DIDs (one per line)"
                className="w-full bg-white/[0.02] text-[10px] font-mono px-3 py-2 outline-none placeholder:text-neutral-700 resize-none"
                rows={3}
              />
              <div className="flex gap-2">
                <button onClick={createGroup} className="flex-1 text-[10px] font-mono uppercase tracking-wider py-1.5 border border-[#d4af37]/30 text-[#d4af37]">
                  Create
                </button>
                <button onClick={() => setCreateMode(null)} className="text-[10px] font-mono text-neutral-600 hover:text-white px-2">
                  Cancel
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Channel list */}
        <div className="flex-1 overflow-y-auto">
          {channels.length === 0 && (
            <p className="px-6 text-xs text-neutral-700 font-light">No conversations yet</p>
          )}
          {channels.map((ch) => (
            <button
              key={ch.id}
              onClick={() => { setSelected(ch.id); setReplyTo(null); }}
              className={cn(
                "w-full text-left px-6 py-4 transition-colors duration-150 border-b border-white/[0.03]",
                selected === ch.id ? "bg-white/[0.02]" : "hover:bg-white/[0.01]"
              )}
            >
              <div className="flex justify-between items-baseline mb-1.5">
                <span className="text-xs font-mono text-neutral-500 truncate max-w-[140px]">
                  {channelDisplayName(ch, userDid)}
                </span>
                <span className={cn(
                  "text-[10px] font-mono uppercase tracking-wider",
                  ch.kind === "direct" ? "text-neutral-700" : ch.kind === "group" ? "text-[#d4af37]/50" : "text-emerald-700"
                )}>
                  {ch.kind === "direct" ? "DM" : ch.kind === "group" ? "GRP" : "PUB"}
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
        {/* Header */}
        <div className="px-8 py-5 border-b border-white/[0.06] flex items-center justify-between">
          {selectedChannel ? (
            <div>
              <p className="text-sm font-light">
                {channelDisplayName(selectedChannel, userDid)}
              </p>
              <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                {selectedChannel.members.length} members
                {selectedChannel.kind !== "direct" && ` \u00b7 ${selectedChannel.kind}`}
              </p>
            </div>
          ) : (
            <p className="text-xs text-neutral-700 font-light">Select a conversation</p>
          )}
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto px-8 py-6 space-y-4">
          {!selected && (
            <div className="flex items-center justify-center h-full">
              <div className="text-center">
                <p className="text-sm text-neutral-700 font-light">Select a conversation or create a new one</p>
                {!userDid && (
                  <p className="text-[10px] text-red-500/60 font-mono mt-2">Set your identity in Settings first</p>
                )}
              </div>
            </div>
          )}
          {messages.map((msg) => {
            const isSelf = msg.sender === userDid;
            return (
              <div key={msg.id} className={cn("max-w-[70%] group", isSelf ? "ml-auto" : "")}>
                {!isSelf && (
                  <p className="text-[10px] font-mono text-neutral-700 mb-1">
                    {truncateDid(msg.sender)}
                  </p>
                )}
                {msg.reply_to && (
                  <div className="text-[10px] font-mono text-neutral-700 mb-1 pl-3 border-l border-white/[0.06]">
                    Replying to message
                  </div>
                )}
                <div
                  className={cn(
                    "px-4 py-3 text-sm font-light",
                    isSelf ? "bg-white/[0.04] text-white" : "bg-white/[0.02] text-neutral-300"
                  )}
                >
                  {msg.content}
                </div>
                <div className={cn(
                  "flex items-center gap-3 mt-1",
                  isSelf ? "justify-end" : ""
                )}>
                  <span className="text-[10px] text-neutral-700">{timeAgo(msg.timestamp)}</span>
                  <button
                    onClick={() => setReplyTo(msg)}
                    className="text-[10px] font-mono text-neutral-800 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity"
                  >
                    reply
                  </button>
                  {isSelf && (
                    <button
                      onClick={() => deleteMessage(msg.id)}
                      className="text-[10px] font-mono text-neutral-800 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      delete
                    </button>
                  )}
                </div>
              </div>
            );
          })}
          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        {selected && (
          <div className="px-8 py-5 border-t border-white/[0.06]">
            {replyTo && (
              <div className="flex items-center justify-between mb-3 pb-3 border-b border-white/[0.04]">
                <span className="text-[10px] font-mono text-neutral-600 truncate">
                  Replying to {truncateDid(replyTo.sender)}: {replyTo.content.slice(0, 50)}
                  {replyTo.content.length > 50 ? "..." : ""}
                </span>
                <button
                  onClick={() => setReplyTo(null)}
                  className="text-[10px] font-mono text-neutral-700 hover:text-white ml-3"
                >
                  Cancel
                </button>
              </div>
            )}
            <div className="flex gap-4">
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    sendMessage();
                  }
                }}
                placeholder="Type a message..."
                className="flex-1 bg-transparent text-sm font-light outline-none placeholder:text-neutral-700"
              />
              <button
                onClick={sendMessage}
                disabled={!input.trim()}
                className="text-xs font-mono uppercase tracking-wider text-neutral-500 hover:text-[#d4af37] transition-colors duration-150 disabled:opacity-30"
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
