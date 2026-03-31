"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import {
  files,
  identity,
  type FileManifestResponse,
  type FileStoreStats,
} from "@/lib/api";
import { EmptyState, FilesIllustration } from "@/components/empty-state";
import { useToast } from "@/components/toast";
import { PageHeader } from "@/components/page-header";

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function mimeIcon(mime: string): string {
  if (mime.startsWith("image/")) return "\u25A3";
  if (mime.startsWith("text/")) return "\u2261";
  if (mime.startsWith("application/pdf")) return "\u25A0";
  if (mime.startsWith("application/json")) return "{ }";
  return "\u25CB";
}

export default function FilesPage() {
  const [fileList, setFileList] = useState<FileManifestResponse[]>([]);
  const [stats, setStats] = useState<FileStoreStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [userDid, setUserDid] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const { toast } = useToast();

  useEffect(() => {
    const stored = localStorage.getItem("nous_did");
    if (stored) setUserDid(stored);
  }, []);

  async function ensureIdentity(): Promise<string> {
    if (userDid) return userDid;
    const id = await identity.create("Nous User");
    localStorage.setItem("nous_did", id.did);
    setUserDid(id.did);
    return id.did;
  }

  const loadFiles = useCallback(async () => {
    if (!userDid) {
      setLoading(false);
      return;
    }
    try {
      const [data, storeStats] = await Promise.all([
        files.list(userDid),
        files.stats(),
      ]);
      setFileList(data.files);
      setStats(storeStats);
    } catch (e) {
      toast({ title: "Failed to load files", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [userDid]);

  useEffect(() => {
    loadFiles();
  }, [loadFiles]);

  async function handleUpload(file: File) {
    setUploading(true);
    try {
      const did = await ensureIdentity();
      const buffer = await file.arrayBuffer();
      const base64 = btoa(
        new Uint8Array(buffer).reduce(
          (data, byte) => data + String.fromCharCode(byte),
          ""
        )
      );
      await files.upload({
        name: file.name,
        mime_type: file.type || "application/octet-stream",
        owner: did,
        data_base64: base64,
      });
      await loadFiles();
      toast({ title: "File uploaded", description: file.name, variant: "success" });
    } catch (e) {
      toast({ title: "Upload failed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setUploading(false);
    }
  }

  async function handleDelete(name: string) {
    if (!userDid) return;
    try {
      await files.delete(name, userDid);
      setSelectedFile(null);
      await loadFiles();
      toast({ title: "File deleted", description: name });
    } catch (e) {
      toast({ title: "Delete failed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  async function handleDownload(f: FileManifestResponse) {
    try {
      const content = await files.get(f.id["0"]);
      const binary = atob(content.data_base64);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
      }
      const blob = new Blob([bytes], { type: f.mime_type });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = f.name;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      toast({ title: "Download failed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  return (
    <div className="p-8 max-w-4xl">
      <PageHeader title="Files" subtitle="Content-addressed storage. Versioned. Deduplicated." />

      {/* Stats bar */}
      {stats && (
        <div className="flex gap-8 mb-10 pb-4 border-b border-white/[0.06]">
          {[
            { label: "Files", value: String(stats.total_files) },
            { label: "Stored", value: formatBytes(stats.stored_bytes) },
            { label: "Chunks", value: String(stats.total_chunks) },
            { label: "Dedup", value: `${stats.dedup_ratio.toFixed(1)}x` },
          ].map((s) => (
            <div key={s.label}>
              <p className="text-[10px] font-mono uppercase tracking-wider text-neutral-600">
                {s.label}
              </p>
              <p className="text-sm font-light mt-0.5">{s.value}</p>
            </div>
          ))}
        </div>
      )}

      {/* Upload area */}
      <div className="flex items-center justify-between mb-8">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
          {fileList.length} File{fileList.length !== 1 ? "s" : ""}
        </h2>
        <div className="flex gap-3">
          <input
            ref={fileInput}
            type="file"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) handleUpload(f);
              e.target.value = "";
            }}
          />
          <Button
            variant="outline"
            size="sm"
            disabled={uploading}
            onClick={() => fileInput.current?.click()}
            className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
          >
            {uploading ? "Uploading..." : "Upload File"}
          </Button>
        </div>
      </div>

      {!userDid ? (
        <div className="text-sm text-neutral-600 font-light">
          <button
            onClick={async () => {
              await ensureIdentity();
              loadFiles();
            }}
            className="text-[#d4af37] hover:underline"
          >
            Create an identity
          </button>{" "}
          to start using file storage.
        </div>
      ) : loading ? (
        <div className="space-y-px">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="p-4">
              <div className="flex items-center gap-4">
                <Skeleton className="h-4 w-6" />
                <div className="flex-1 min-w-0">
                  <Skeleton className="h-3.5 w-48 mb-2" />
                  <div className="flex gap-4">
                    <Skeleton className="h-2.5 w-12" />
                    <Skeleton className="h-2.5 w-8" />
                    <Skeleton className="h-2.5 w-24" />
                  </div>
                </div>
                <Skeleton className="h-2.5 w-24" />
              </div>
            </div>
          ))}
        </div>
      ) : fileList.length === 0 ? (
        <EmptyState
          icon={<FilesIllustration />}
          title="No files yet"
          description="Upload your first file. Content is chunked, deduplicated, and addressed by hash."
          action={
            <button
              onClick={() => fileInput.current?.click()}
              className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
            >
              Upload File
            </button>
          }
        />
      ) : (
        <div className="space-y-px stagger-in">
          {fileList.map((f) => (
            <Card
              key={f.id["0"]}
              className={cn(
                "bg-transparent border-0 rounded-none cursor-pointer transition-colors duration-150",
                selectedFile === f.id["0"]
                  ? "bg-white/[0.02]"
                  : "hover:bg-white/[0.01]"
              )}
              onClick={() =>
                setSelectedFile(selectedFile === f.id["0"] ? null : f.id["0"])
              }
            >
              <CardContent className="p-4">
                <div className="flex items-center gap-4">
                  <span className="text-neutral-600 font-mono text-sm w-6 text-center">
                    {mimeIcon(f.mime_type)}
                  </span>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-light truncate">{f.name}</p>
                    <div className="flex gap-4 mt-0.5">
                      <span className="text-[10px] font-mono text-neutral-700">
                        {formatBytes(f.total_size)}
                      </span>
                      <span className="text-[10px] font-mono text-neutral-700">
                        v{f.version}
                      </span>
                      <span className="text-[10px] font-mono text-neutral-700">
                        {f.mime_type}
                      </span>
                    </div>
                  </div>
                  <span className="text-[10px] font-mono text-neutral-700 shrink-0">
                    {formatDate(f.created_at)}
                  </span>
                </div>

                {selectedFile === f.id["0"] && (
                  <div className="mt-4 pt-4 border-t border-white/[0.04] ml-10">
                    <div className="grid grid-cols-2 gap-y-2 mb-4">
                      <p className="text-[10px] font-mono text-neutral-600">
                        Content ID
                      </p>
                      <p className="text-[10px] font-mono text-neutral-500 truncate">
                        {f.content_hash.slice(0, 32)}...
                      </p>
                      <p className="text-[10px] font-mono text-neutral-600">
                        Chunks
                      </p>
                      <p className="text-[10px] font-mono text-neutral-500">
                        {f.chunk_count}
                      </p>
                    </div>
                    <div className="flex gap-3">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDownload(f);
                        }}
                        className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
                      >
                        Download
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDelete(f.name);
                        }}
                        className="text-xs font-mono uppercase tracking-wider border-red-900/30 text-red-700 hover:bg-red-950"
                      >
                        Delete
                      </Button>
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
