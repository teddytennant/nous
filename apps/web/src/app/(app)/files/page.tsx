"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { DataTable, type Column } from "@/components/ui/data-table";
import { Tooltip } from "@/components/ui/tooltip";
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
import { usePageShortcuts } from "@/components/keyboard-shortcuts";

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
  const [search, setSearch] = useState("");
  const fileInput = useRef<HTMLInputElement>(null);
  const { toast } = useToast();

  usePageShortcuts({
    u: () => fileInput.current?.click(),
  });

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
  }, [userDid, toast]);

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
    <div className="p-4 sm:p-8 max-w-4xl">
      <PageHeader title="Files" subtitle="Content-addressed storage. Versioned. Deduplicated." />

      {/* Stats bar */}
      {stats && (
        <div className="grid grid-cols-2 sm:flex gap-4 sm:gap-8 mb-8 sm:mb-10 pb-4 border-b border-white/[0.06]">
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

      {/* Upload + search */}
      <div className="flex flex-col sm:flex-row gap-3 sm:items-center sm:justify-between mb-6">
        <div className="flex items-center gap-4">
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
            {fileList.length} File{fileList.length !== 1 ? "s" : ""}
          </h2>
          {fileList.length > 0 && (
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Filter by name..."
              className="bg-white/[0.02] border border-white/[0.06] rounded-sm text-sm font-light px-3 py-1.5 outline-none placeholder:text-neutral-700 focus:border-[#d4af37]/40 transition-colors duration-200 w-48"
            />
          )}
        </div>
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
            <div key={i} className="py-3">
              <div className="flex items-center gap-4">
                <Skeleton className="h-4 w-6" />
                <Skeleton className="h-3.5 w-48 flex-1 max-w-[200px]" />
                <Skeleton className="h-2.5 w-16" />
                <Skeleton className="h-2.5 w-20 hidden sm:block" />
                <Skeleton className="h-2.5 w-12 hidden md:block" />
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
      ) : (() => {
        const filtered = fileList.filter((f) =>
          !search || f.name.toLowerCase().includes(search.toLowerCase())
        );

        const fileColumns: Column<FileManifestResponse>[] = [
          {
            id: "name",
            header: "Name",
            sortable: true,
            compare: (a, b) => a.name.localeCompare(b.name),
            cell: (f) => (
              <div className="flex items-center gap-3 min-w-0">
                <span className="text-neutral-600 font-mono text-sm w-5 text-center shrink-0">
                  {mimeIcon(f.mime_type)}
                </span>
                <span className="text-sm font-light truncate">{f.name}</span>
              </div>
            ),
          },
          {
            id: "size",
            header: "Size",
            align: "right",
            sortable: true,
            compare: (a, b) => a.total_size - b.total_size,
            cell: (f) => (
              <span className="text-xs font-mono text-neutral-500">
                {formatBytes(f.total_size)}
              </span>
            ),
          },
          {
            id: "type",
            header: "Type",
            sortable: true,
            compare: (a, b) => a.mime_type.localeCompare(b.mime_type),
            hideBelow: "sm",
            cell: (f) => (
              <Tooltip content={f.mime_type}>
                <span className="text-xs font-mono text-neutral-600 cursor-default">
                  {f.mime_type.split("/")[1] ?? f.mime_type}
                </span>
              </Tooltip>
            ),
          },
          {
            id: "version",
            header: "Ver",
            align: "center",
            hideBelow: "md",
            cell: (f) => (
              <span className="text-xs font-mono text-neutral-600">
                v{f.version}
              </span>
            ),
          },
          {
            id: "date",
            header: "Date",
            align: "right",
            sortable: true,
            compare: (a, b) =>
              new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
            cell: (f) => (
              <span className="text-[11px] font-mono text-neutral-600">
                {formatDate(f.created_at)}
              </span>
            ),
          },
          {
            id: "actions",
            header: "",
            align: "right",
            cell: (f) => (
              <div className="flex gap-2 justify-end opacity-0 group-hover:opacity-100 transition-opacity duration-150">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDownload(f);
                  }}
                  className="text-[10px] font-mono text-neutral-600 hover:text-[#d4af37] transition-colors duration-150"
                >
                  download
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(f.name);
                  }}
                  className="text-[10px] font-mono text-neutral-600 hover:text-red-400 transition-colors duration-150"
                >
                  delete
                </button>
              </div>
            ),
          },
        ];

        return filtered.length === 0 ? (
          <EmptyState
            icon={<FilesIllustration />}
            title="No matching files"
            description={`No files match "${search}".`}
            action={
              <button
                onClick={() => setSearch("")}
                className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-400 hover:text-white hover:border-white/20 transition-all duration-150"
              >
                Clear filter
              </button>
            }
          />
        ) : (
          <>
            <DataTable
              columns={fileColumns}
              data={filtered}
              rowKey={(f) => f.id["0"]}
              defaultSortId="date"
              defaultSortDir="desc"
              onRowClick={(f) =>
                setSelectedFile(selectedFile === f.id["0"] ? null : f.id["0"])
              }
              isRowSelected={(f) => selectedFile === f.id["0"]}
            />
            {/* Expanded detail panel below table */}
            {selectedFile && (() => {
              const f = fileList.find((x) => x.id["0"] === selectedFile);
              if (!f) return null;
              return (
                <div className="mt-0 border-t border-white/[0.04] bg-white/[0.01] px-4 py-4">
                  <div className="flex items-start justify-between gap-6">
                    <div className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5">
                      <span className="text-[10px] font-mono text-neutral-600">Content ID</span>
                      <Tooltip content={f.content_hash}>
                        <span className="text-[10px] font-mono text-neutral-500 truncate max-w-[240px] cursor-default">
                          {f.content_hash.slice(0, 32)}...
                        </span>
                      </Tooltip>
                      <span className="text-[10px] font-mono text-neutral-600">Chunks</span>
                      <span className="text-[10px] font-mono text-neutral-500">{f.chunk_count}</span>
                      <span className="text-[10px] font-mono text-neutral-600">Full type</span>
                      <span className="text-[10px] font-mono text-neutral-500">{f.mime_type}</span>
                    </div>
                    <div className="flex gap-3 shrink-0">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleDownload(f)}
                        className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
                      >
                        Download
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleDelete(f.name)}
                        className="text-xs font-mono uppercase tracking-wider border-red-900/30 text-red-700 hover:bg-red-950"
                      >
                        Delete
                      </Button>
                    </div>
                  </div>
                </div>
              );
            })()}
          </>
        );
      })()}
    </div>
  );
}
