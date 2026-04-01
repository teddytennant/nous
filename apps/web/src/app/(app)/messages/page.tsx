"use client";

import { useCallback, useEffect, useRef, useState, startTransition } from "react";
import { ArrowLeft, Send, Lock } from "lucide-react";
import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";
import { messaging, type ChannelResponse, type MessageResponse } from "@/lib/api";
import { useRealtime } from "@/lib/use-realtime";
import { useToast } from "@/components/toast";
import { EmptyState, MessagesIllustration } from "@/components/empty-state";
import { usePageShortcuts, useListNavigation } from "@/components/keyboard-shortcuts";
import { Avatar } from "@/components/avatar";

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
  const [loading, setLoading] = useState(true);
  const [createMode, setCreateMode] = useState<CreateMode>(null);
  const [newDmDid, setNewDmDid] = useState("");
  const [newGroupName, setNewGroupName] = useState("");
  const [newGroupMembers, setNewGroupMembers] = useState("");
  const [replyTo, setReplyTo] = useState<MessageResponse | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const { toast } = useToast();
  const userDid = typeof window !== "undefined" ? localStorage.getItem("nous_did") || "" : "";

  usePageShortcuts({
    n: () => setCreateMode("dm"),
  });

  const {
    selectedIndex: highlightedIndex,
    setSelectedIndex: setHighlightedIndex,
    containerRef: channelListRef,
  } = useListNavigation({
    itemCount: channels.length,
    onActivate: (index) => {
      const ch = channels[index];
      if (ch) {
        setSelected(ch.id);
        setReplyTo(null);
      }
    },
  });

  const fetchChannels = useCallback(async () => {
    if (!userDid) { setLoading(false); return; }
    try {
      const chs = await messaging.listChannels(userDid);
      setChannels(chs);
    } catch (e) {
      toast({ title: "API offline", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [userDid, toast]);

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
      toast({ title: "Conversation created", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to create channel", description: e instanceof Error ? e.message : undefined, variant: "error" });
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
      toast({ title: "Group created", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to create group", description: e instanceof Error ? e.message : undefined, variant: "error" });
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
      toast({ title: "Failed to send", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  async function deleteMessage(messageId: string) {
    try {
      await messaging.deleteMessage(messageId);
      setMessages((prev) => prev.filter((m) => m.id !== messageId));
      toast({ title: "Message deleted" });
    } catch (e) {
      toast({ title: "Failed to delete", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  const selectedChannel = channels.find((c) => c.id === selected);

  return (
    <div className="flex h-[calc(100dvh-3.5rem)] md:h-screen">
      {/* Channel list — full-width on mobile, fixed sidebar on desktop */}
      <div className={cn(
        "w-full md:w-72 border-r border-white/[0.06] flex flex-col shrink-0",
        selected ? "hidden md:flex" : "flex"
      )}>
        <div className="p-4 sm:p-6 pb-4">
          <h1 className="text-lg font-extralight tracking-[-0.02em]">Messages</h1>
          <p className="text-[10px] font-mono text-neutral-600 mt-1 uppercase tracking-wider">
            E2E encrypted
          </p>
        </div>

        {/* Create channel */}
        <div className="px-4 sm:px-6 pb-4">
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
        <div ref={channelListRef} className="flex-1 overflow-y-auto">
          {loading ? (
            <div>
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="px-4 sm:px-6 py-3.5 border-b border-white/[0.03]">
                  <div className="flex items-center gap-3">
                    <Skeleton className="w-7 h-7 rounded-full shrink-0" />
                    <div className="flex-1">
                      <div className="flex justify-between items-baseline mb-1">
                        <Skeleton className="h-3 w-28" />
                        <Skeleton className="h-2.5 w-8" />
                      </div>
                      <Skeleton className="h-2.5 w-16" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : channels.length === 0 ? (
            <div className="px-6 py-8 text-center">
              <MessagesIllustration />
              <p className="text-xs text-neutral-600 font-light mt-4">No conversations yet</p>
              <p className="text-[10px] text-neutral-700 font-light mt-1">Start a DM or create a group above</p>
            </div>
          ) : null}
          {channels.map((ch, i) => {
            const isHighlighted = i === highlightedIndex;
            const avatarDid = ch.kind === "direct"
              ? (ch.members.find((m) => m !== userDid) || ch.id)
              : ch.id;
            return (
            <button
              key={ch.id}
              data-list-item
              onClick={() => { setSelected(ch.id); setReplyTo(null); setHighlightedIndex(i); }}
              className={cn(
                "relative w-full text-left px-4 sm:px-6 py-3.5 transition-colors duration-150 border-b border-white/[0.03]",
                selected === ch.id ? "bg-white/[0.02]" : "hover:bg-white/[0.01]",
                isHighlighted && selected !== ch.id && "bg-[#d4af37]/[0.015]"
              )}
            >
              {isHighlighted && (
                <div className="absolute left-0 top-0 bottom-0 w-0.5 bg-[#d4af37] rounded-full" />
              )}
              <div className="flex items-center gap-3">
                <Avatar did={avatarDid} size="sm" />
                <div className="flex-1 min-w-0">
                  <div className="flex justify-between items-baseline mb-0.5">
                    <span className="text-xs font-mono text-neutral-500 truncate">
                      {channelDisplayName(ch, userDid)}
                    </span>
                    <span className={cn(
                      "text-[10px] font-mono uppercase tracking-wider shrink-0 ml-2",
                      ch.kind === "direct" ? "text-neutral-700" : ch.kind === "group" ? "text-[#d4af37]/50" : "text-emerald-700"
                    )}>
                      {ch.kind === "direct" ? "DM" : ch.kind === "group" ? "GRP" : "PUB"}
                    </span>
                  </div>
                  <p className="text-[10px] font-mono text-neutral-700">
                    {ch.members.length} member{ch.members.length !== 1 ? "s" : ""}
                  </p>
                </div>
              </div>
            </button>
            );
          })}
        </div>
      </div>

      {/* Chat view — hidden on mobile when no channel selected */}
      <div className={cn(
        "flex-1 flex flex-col min-w-0",
        !selected ? "hidden md:flex" : "flex"
      )}>
        {/* Header */}
        <div className="px-4 sm:px-8 py-4 sm:py-5 border-b border-white/[0.06] flex items-center justify-between">
          {selectedChannel ? (
            <div className="flex items-center gap-3">
              <button
                onClick={() => setSelected(null)}
                className="md:hidden p-1 -ml-1 rounded-sm hover:bg-white/[0.04] transition-colors"
                aria-label="Back to channels"
              >
                <ArrowLeft className="w-4 h-4 text-neutral-400" />
              </button>
              <div>
                <p className="text-sm font-light">
                  {channelDisplayName(selectedChannel, userDid)}
                </p>
                <div className="flex items-center gap-2 mt-0.5">
                  <Lock size={9} className="text-emerald-600" />
                  <p className="text-[10px] font-mono text-neutral-700">
                    {selectedChannel.members.length} members
                    {selectedChannel.kind !== "direct" && ` \u00b7 ${selectedChannel.kind}`}
                    {" \u00b7 "}
                    <span className="text-emerald-700">encrypted</span>
                  </p>
                </div>
              </div>
            </div>
          ) : (
            <p className="text-xs text-neutral-700 font-light">Select a conversation</p>
          )}
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto px-4 sm:px-8 py-4 sm:py-6 space-y-4">
          {!selected && (
            <div className="flex items-center justify-center h-full">
              <EmptyState
                icon={<MessagesIllustration />}
                title="Select a conversation"
                description={userDid ? "Choose a conversation from the sidebar or start a new one." : "Set your identity in Settings to start messaging."}
              />
            </div>
          )}
          {messages.map((msg) => {
            const isSelf = msg.sender === userDid;
            return (
              <div key={msg.id} className={cn("max-w-[70%] group chat-msg-enter", isSelf ? "ml-auto" : "")}>
                {msg.reply_to && (
                  <div className={cn("text-[10px] font-mono text-neutral-700 mb-1 pl-3 border-l border-white/[0.06]", !isSelf && "ml-9")}>
                    Replying to message
                  </div>
                )}
                <div className={cn("flex gap-2", isSelf && "flex-row-reverse")}>
                  {!isSelf && (
                    <Avatar did={msg.sender} size="xs" className="mt-1" />
                  )}
                  <div className="flex-1 min-w-0">
                    {!isSelf && (
                      <p className="text-[10px] font-mono text-neutral-700 mb-1">
                        {truncateDid(msg.sender)}
                      </p>
                    )}
                    <div
                      className={cn(
                        "px-4 py-3 text-sm font-light rounded-sm whitespace-pre-wrap",
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
                </div>
              </div>
            );
          })}
          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        {selected && (
          <div className="px-4 sm:px-8 py-4 sm:py-5 border-t border-white/[0.06]">
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
            <div className="flex items-end gap-3">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => {
                  setInput(e.target.value);
                  e.target.style.height = "auto";
                  e.target.style.height = Math.min(e.target.scrollHeight, 120) + "px";
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    sendMessage();
                    requestAnimationFrame(() => {
                      if (inputRef.current) inputRef.current.style.height = "auto";
                    });
                  }
                }}
                placeholder="Type a message..."
                className="flex-1 bg-transparent text-sm font-light outline-none placeholder:text-neutral-700 resize-none min-h-[24px] max-h-[120px]"
                rows={1}
              />
              <button
                onClick={() => {
                  sendMessage();
                  requestAnimationFrame(() => {
                    if (inputRef.current) inputRef.current.style.height = "auto";
                  });
                }}
                disabled={!input.trim()}
                className={cn(
                  "shrink-0 w-8 h-8 flex items-center justify-center rounded-sm transition-all duration-150",
                  input.trim()
                    ? "bg-[#d4af37] text-black hover:bg-[#c4a030]"
                    : "bg-white/[0.04] text-neutral-700 cursor-not-allowed"
                )}
              >
                <Send size={14} />
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
