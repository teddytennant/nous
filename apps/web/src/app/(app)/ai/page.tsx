"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  ai,
  type AgentResponse,
  type ChatResponse,
  type ConversationResponse,
  type AIMessage,
} from "@/lib/api";
import { EmptyState, AIIllustration, ChatIllustration, ConversationsIllustration } from "@/components/empty-state";
import { PageHeader } from "@/components/page-header";

type ViewMode = "chat" | "agents" | "conversations";

export default function AIPage() {
  const [mode, setMode] = useState<ViewMode>("chat");
  const [agents, setAgents] = useState<AgentResponse[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<AgentResponse | null>(
    null
  );
  const [conversations, setConversations] = useState<ConversationResponse[]>(
    []
  );
  const [messages, setMessages] = useState<AIMessage[]>([]);
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Agent creation
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [newPrompt, setNewPrompt] = useState("");

  const messagesEnd = useRef<HTMLDivElement>(null);

  const loadAgents = useCallback(async () => {
    try {
      const data = await ai.listAgents();
      setAgents(data.agents);
      if (data.agents.length > 0 && !selectedAgent) {
        setSelectedAgent(data.agents[0]);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load agents");
    } finally {
      setLoading(false);
    }
  }, [selectedAgent]);

  const loadConversations = useCallback(async () => {
    try {
      const data = await ai.listConversations({ limit: 50 });
      setConversations(data);
    } catch {
      // Silently fail — conversations may not exist yet.
    }
  }, []);

  const loadMessages = useCallback(async (convId: string) => {
    try {
      const data = await ai.getConversation(convId);
      setMessages(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load messages");
    }
  }, []);

  useEffect(() => {
    loadAgents();
    loadConversations();
  }, [loadAgents, loadConversations]);

  useEffect(() => {
    messagesEnd.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function handleSend() {
    if (!input.trim() || sending || !selectedAgent) return;
    setSending(true);
    setError(null);

    // Optimistically add user message.
    const userMsg: AIMessage = {
      id: `temp-${Date.now()}`,
      role: "user",
      content: input,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);
    const userInput = input;
    setInput("");

    try {
      const res: ChatResponse = await ai.chat({
        agent_id: selectedAgent.id,
        message: userInput,
        conversation_id: conversationId ?? undefined,
      });

      if (!conversationId) {
        setConversationId(res.conversation_id);
      }

      // Add assistant response.
      const asstMsg: AIMessage = {
        id: `resp-${Date.now()}`,
        role: "assistant",
        content: res.response,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, asstMsg]);
      loadConversations();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to send message");
    } finally {
      setSending(false);
    }
  }

  async function handleCreateAgent() {
    if (!newName.trim()) return;
    try {
      const agent = await ai.createAgent({
        name: newName,
        system_prompt: newPrompt || undefined,
      });
      setAgents((prev) => [...prev, agent]);
      setSelectedAgent(agent);
      setNewName("");
      setNewPrompt("");
      setShowCreate(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create agent");
    }
  }

  async function handleDeleteAgent(agentId: string) {
    try {
      await ai.deleteAgent(agentId);
      setAgents((prev) => prev.filter((a) => a.id !== agentId));
      if (selectedAgent?.id === agentId) {
        setSelectedAgent(agents.find((a) => a.id !== agentId) ?? null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to delete agent");
    }
  }

  function startNewConversation() {
    setConversationId(null);
    setMessages([]);
  }

  async function openConversation(convId: string) {
    setConversationId(convId);
    setMode("chat");
    await loadMessages(convId);
  }

  function formatTime(iso: string): string {
    const date = new Date(iso);
    return date.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  return (
    <div className="p-8 max-w-4xl">
      <PageHeader title="AI" subtitle="Local-first inference. Your agents, your data, your sovereignty." />

      {/* Navigation tabs */}
      <div className="flex items-center gap-6 mb-10">
        {(["chat", "agents", "conversations"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setMode(tab)}
            className={`text-xs font-mono uppercase tracking-[0.2em] pb-2 transition-colors duration-150 ${
              mode === tab
                ? "text-[#d4af37] border-b border-[#d4af37]"
                : "text-neutral-600 hover:text-neutral-400"
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {error && (
        <div className="text-xs text-red-500/70 font-mono mb-6 px-1 flex items-center justify-between">
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            className="text-neutral-600 hover:text-white ml-4"
          >
            dismiss
          </button>
        </div>
      )}

      {/* ── Chat View ─────────────────────────────────────── */}
      {mode === "chat" && (
        <div>
          {/* Agent selector + New chat */}
          <div className="flex items-center justify-between mb-8">
            <div className="flex items-center gap-4">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Agent
              </span>
              {agents.length > 0 ? (
                <select
                  value={selectedAgent?.id ?? ""}
                  onChange={(e) => {
                    const agent = agents.find((a) => a.id === e.target.value);
                    if (agent) {
                      setSelectedAgent(agent);
                      startNewConversation();
                    }
                  }}
                  className="bg-transparent text-sm font-light border border-white/[0.06] px-3 py-1.5 outline-none focus:border-[#d4af37] transition-colors"
                >
                  {agents.map((a) => (
                    <option key={a.id} value={a.id} className="bg-black">
                      {a.name}
                    </option>
                  ))}
                </select>
              ) : (
                <span className="text-xs text-neutral-600 font-light">
                  No agents yet
                </span>
              )}
            </div>
            <button
              onClick={startNewConversation}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
            >
              New chat
            </button>
          </div>

          {/* Messages */}
          <div className="min-h-[400px] max-h-[600px] overflow-y-auto mb-8 space-y-6">
            {messages.length === 0 && !loading && (
              <EmptyState
                icon={<ChatIllustration />}
                title={selectedAgent ? `Chat with ${selectedAgent.name}` : "No agent selected"}
                description={selectedAgent ? "Type a message below to start a conversation. All inference runs locally." : "Create an agent in the Agents tab to begin."}
              />
            )}
            {messages
              .filter((m) => m.role !== "system")
              .map((msg) => (
                <div
                  key={msg.id}
                  className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
                >
                  <div
                    className={`max-w-[85%] ${
                      msg.role === "user"
                        ? "text-right"
                        : ""
                    }`}
                  >
                    <div className="flex items-baseline gap-3 mb-1">
                      <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                        {msg.role === "user" ? "You" : selectedAgent?.name ?? "Assistant"}
                      </span>
                      <span className="text-[10px] text-neutral-700">
                        {formatTime(msg.timestamp)}
                      </span>
                    </div>
                    <div
                      className={`text-sm font-light leading-relaxed ${
                        msg.role === "user"
                          ? "text-neutral-300"
                          : "text-neutral-100"
                      }`}
                    >
                      <p className="whitespace-pre-wrap">{msg.content}</p>
                    </div>
                  </div>
                </div>
              ))}
            {sending && (
              <div className="flex justify-start">
                <span className="text-xs text-neutral-600 font-mono animate-pulse">
                  Thinking...
                </span>
              </div>
            )}
            <div ref={messagesEnd} />
          </div>

          {/* Input */}
          <div className="border border-white/[0.06] p-4">
            <div className="flex gap-3">
              <input
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                  }
                }}
                placeholder={
                  selectedAgent
                    ? "Type a message..."
                    : "Create an agent first"
                }
                disabled={!selectedAgent || sending}
                className="flex-1 bg-transparent text-sm font-light outline-none placeholder:text-neutral-700 disabled:opacity-30"
              />
              <Button
                onClick={handleSend}
                disabled={!input.trim() || sending || !selectedAgent}
                variant="outline"
                size="sm"
                className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37] disabled:opacity-30"
              >
                Send
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Agents View ───────────────────────────────────── */}
      {mode === "agents" && (
        <div>
          <div className="flex items-center justify-between mb-8">
            <span className="text-xs font-mono text-neutral-600">
              {agents.length} agent{agents.length !== 1 ? "s" : ""}
            </span>
            <button
              onClick={() => setShowCreate(!showCreate)}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
            >
              {showCreate ? "Cancel" : "New agent"}
            </button>
          </div>

          {showCreate && (
            <Card className="bg-transparent border border-white/[0.06] rounded-none mb-8">
              <CardContent className="p-5 space-y-4">
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="Agent name"
                  className="w-full bg-transparent text-sm font-light border-b border-white/[0.06] pb-2 outline-none placeholder:text-neutral-700 focus:border-[#d4af37] transition-colors"
                />
                <textarea
                  value={newPrompt}
                  onChange={(e) => setNewPrompt(e.target.value)}
                  placeholder="System prompt (optional)"
                  rows={3}
                  className="w-full bg-transparent text-sm font-light resize-none outline-none placeholder:text-neutral-700 border-b border-white/[0.06] pb-2 focus:border-[#d4af37] transition-colors"
                />
                <div className="flex justify-end">
                  <Button
                    onClick={handleCreateAgent}
                    disabled={!newName.trim()}
                    variant="outline"
                    size="sm"
                    className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37] disabled:opacity-30"
                  >
                    Create
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {loading ? (
            <div className="space-y-px">
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="border-b border-white/[0.04] pb-6 mb-6">
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <Skeleton className="h-3.5 w-32 mb-2" />
                      <Skeleton className="h-2.5 w-28 mb-3" />
                      <Skeleton className="h-3 w-full" />
                      <Skeleton className="h-3 w-4/5 mt-1" />
                    </div>
                    <div className="flex gap-4 ml-4">
                      <Skeleton className="h-2.5 w-10" />
                      <Skeleton className="h-2.5 w-12" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : agents.length === 0 ? (
            <EmptyState
              icon={<AIIllustration />}
              title="No agents yet"
              description="Create an AI agent with a custom system prompt. Agents run locally with full sovereignty over your data."
              action={
                <button
                  onClick={() => setShowCreate(true)}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                >
                  Create Agent
                </button>
              }
            />
          ) : (
            <div className="space-y-px">
              {agents.map((agent) => (
                <Card
                  key={agent.id}
                  className="bg-transparent border-0 rounded-none border-b border-white/[0.04] pb-6 mb-6"
                >
                  <CardContent className="p-0">
                    <div className="flex items-start justify-between">
                      <div>
                        <h3 className="text-sm font-light mb-1">
                          {agent.name}
                        </h3>
                        <p className="text-[10px] font-mono text-neutral-600 mb-2">
                          {agent.model} / temp {agent.temperature}
                        </p>
                        {agent.system_prompt && (
                          <p className="text-xs text-neutral-500 font-light leading-relaxed line-clamp-2">
                            {agent.system_prompt}
                          </p>
                        )}
                        {agent.capabilities.length > 0 && (
                          <div className="flex gap-2 mt-2">
                            {agent.capabilities.map((cap) => (
                              <span
                                key={cap}
                                className="text-[10px] font-mono text-neutral-600 border border-white/[0.04] px-2 py-0.5"
                              >
                                {cap}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                      <div className="flex gap-4">
                        <button
                          onClick={() => {
                            setSelectedAgent(agent);
                            startNewConversation();
                            setMode("chat");
                          }}
                          className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
                        >
                          Chat
                        </button>
                        <button
                          onClick={() => handleDeleteAgent(agent.id)}
                          className="text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-red-400 transition-colors"
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Conversations View ────────────────────────────── */}
      {mode === "conversations" && (
        <div>
          <div className="flex items-center justify-between mb-8">
            <span className="text-xs font-mono text-neutral-600">
              {conversations.length} conversation
              {conversations.length !== 1 ? "s" : ""}
            </span>
            <button
              onClick={loadConversations}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
            >
              Refresh
            </button>
          </div>

          {conversations.length === 0 ? (
            <EmptyState
              icon={<ConversationsIllustration />}
              title="No conversations yet"
              description="Start chatting with an agent in the Chat tab. Your conversation history will appear here."
            />
          ) : (
            <div className="space-y-px">
              {conversations.map((conv) => {
                const agentName =
                  agents.find((a) => a.id === conv.agent_id)?.name ??
                  conv.agent_id;
                return (
                  <Card
                    key={conv.id}
                    className="bg-transparent border-0 rounded-none border-b border-white/[0.04] pb-5 mb-5 cursor-pointer group"
                    onClick={() => openConversation(conv.id)}
                  >
                    <CardContent className="p-0">
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="text-sm font-light group-hover:text-[#d4af37] transition-colors">
                            {agentName}
                          </p>
                          <p className="text-[10px] font-mono text-neutral-600 mt-1">
                            {conv.message_count} messages
                          </p>
                        </div>
                        <span className="text-[10px] text-neutral-700">
                          {new Date(conv.updated_at).toLocaleDateString(
                            "en-US",
                            {
                              month: "short",
                              day: "numeric",
                              hour: "2-digit",
                              minute: "2-digit",
                            }
                          )}
                        </span>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
