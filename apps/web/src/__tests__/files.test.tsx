import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockFilesList = vi.fn();
const mockFilesUpload = vi.fn();
const mockFilesGet = vi.fn();
const mockFilesDelete = vi.fn();
const mockFilesStats = vi.fn();
const mockIdentityCreate = vi.fn();

vi.mock("@/lib/api", () => ({
  files: {
    list: (owner: string) => mockFilesList(owner),
    upload: (data: unknown) => mockFilesUpload(data),
    get: (id: string) => mockFilesGet(id),
    delete: (name: string, owner: string) => mockFilesDelete(name, owner),
    stats: () => mockFilesStats(),
  },
  identity: {
    create: (name?: string) => mockIdentityCreate(name),
  },
}));

const mockToast = vi.fn(() => "toast-id");

vi.mock("@/components/toast", () => ({
  useToast: () => ({ toast: mockToast, dismiss: vi.fn() }),
  ToastProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock("@/components/page-header", () => ({
  PageHeader: ({ title, subtitle }: { title: string; subtitle: string }) => (
    <div data-testid="page-header">
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
  ),
}));

vi.mock("@/components/empty-state", () => ({
  EmptyState: ({
    title,
    description,
    action,
  }: {
    title: string;
    description: string;
    action?: React.ReactNode;
  }) => (
    <div data-testid="empty-state">
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  ),
  FilesIllustration: () => <div data-testid="files-illustration" />,
}));

vi.mock("@/components/keyboard-shortcuts", () => ({
  usePageShortcuts: vi.fn(),
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({
    children,
    content,
  }: {
    children: React.ReactNode;
    content: string;
  }) => <span title={content}>{children}</span>,
}));

vi.mock("@/components/ui/data-table", () => ({
  DataTable: ({
    columns,
    data,
    rowKey,
    onRowClick,
    emptyState,
  }: {
    columns: {
      id: string;
      header: string;
      cell: (row: unknown, i: number) => React.ReactNode;
    }[];
    data: unknown[];
    rowKey: (row: unknown, i: number) => string;
    onRowClick?: (row: unknown, i: number) => void;
    isRowSelected?: (row: unknown) => boolean;
    emptyState?: React.ReactNode;
    defaultSortId?: string;
    defaultSortDir?: string;
  }) =>
    data.length === 0 ? (
      <div data-testid="data-table-empty">{emptyState}</div>
    ) : (
      <table data-testid="data-table">
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={col.id}>{col.header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.map((row, i) => (
            <tr
              key={rowKey(row, i)}
              onClick={() => onRowClick?.(row, i)}
              data-testid={`file-row-${i}`}
            >
              {columns.map((col) => (
                <td key={col.id}>{col.cell(row, i)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    ),
}));

// ── Test data ────────────────────────────────────────────────────────────

const MOCK_DID = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";

const MOCK_FILES = [
  {
    id: { "0": "file-abc123" },
    name: "project.zip",
    mime_type: "application/zip",
    total_size: 1048576,
    chunk_count: 4,
    content_hash: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    owner: MOCK_DID,
    version: 1,
    created_at: new Date(Date.now() - 3600000).toISOString(),
  },
  {
    id: { "0": "file-def456" },
    name: "readme.txt",
    mime_type: "text/plain",
    total_size: 2048,
    chunk_count: 1,
    content_hash: "sha256:fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
    owner: MOCK_DID,
    version: 3,
    created_at: new Date(Date.now() - 86400000).toISOString(),
  },
  {
    id: { "0": "file-ghi789" },
    name: "photo.png",
    mime_type: "image/png",
    total_size: 524288,
    chunk_count: 2,
    content_hash: "sha256:1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff",
    owner: MOCK_DID,
    version: 1,
    created_at: new Date(Date.now() - 172800000).toISOString(),
  },
];

const MOCK_STATS = {
  total_chunks: 7,
  total_manifests: 3,
  total_files: 3,
  stored_bytes: 1574912,
  logical_bytes: 2097152,
  dedup_ratio: 1.3,
};

// ── Helpers ──────────────────────────────────────────────────────────────

import FilesPage from "@/app/(app)/files/page";

function setupDefaults(hasDid = true) {
  mockFilesList.mockResolvedValue({ files: MOCK_FILES, count: 3 });
  mockFilesStats.mockResolvedValue(MOCK_STATS);
  mockFilesUpload.mockResolvedValue(MOCK_FILES[0]);
  mockFilesDelete.mockResolvedValue({ deleted: true, name: "project.zip", freed_bytes: 1048576 });
  mockIdentityCreate.mockResolvedValue({ did: MOCK_DID });

  Object.defineProperty(window, "localStorage", {
    value: {
      getItem: vi.fn((key: string) => {
        if (key === "nous_did") return hasDid ? MOCK_DID : null;
        if (key === "nous_display_name") return "Teddy";
        return null;
      }),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    },
    writable: true,
  });
}

async function renderFiles(hasDid = true) {
  setupDefaults(hasDid);
  render(<FilesPage />);
  await waitFor(() => {
    expect(screen.getByTestId("page-header")).toBeInTheDocument();
  });
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Files page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Page structure ────────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders page header with title and subtitle", async () => {
      await renderFiles();
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Files")).toBeInTheDocument();
      expect(
        within(header).getByText(
          "Content-addressed storage. Versioned. Deduplicated."
        )
      ).toBeInTheDocument();
    });

    it("renders Upload File button", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("Upload File")).toBeInTheDocument();
      });
    });

    it("shows file count header", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("3 Files")).toBeInTheDocument();
      });
    });
  });

  // ── Stats bar ─────────────────────────────────────────────────────────

  describe("Stats bar", () => {
    it("renders stats labels", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("Stored")).toBeInTheDocument();
        expect(screen.getByText("Chunks")).toBeInTheDocument();
        expect(screen.getByText("Dedup")).toBeInTheDocument();
      });
    });

    it("renders stats values", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("3")).toBeInTheDocument();
        expect(screen.getByText("7")).toBeInTheDocument();
        expect(screen.getByText("1.3x")).toBeInTheDocument();
      });
    });

    it("formats stored bytes", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("1.5 MB")).toBeInTheDocument();
      });
    });
  });

  // ── File list ─────────────────────────────────────────────────────────

  describe("File list", () => {
    it("renders file names in table", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
        expect(screen.getByText("readme.txt")).toBeInTheDocument();
        expect(screen.getByText("photo.png")).toBeInTheDocument();
      });
    });

    it("renders file sizes", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("1.0 MB")).toBeInTheDocument();
        expect(screen.getByText("2.0 KB")).toBeInTheDocument();
        expect(screen.getByText("512.0 KB")).toBeInTheDocument();
      });
    });

    it("renders file versions", async () => {
      await renderFiles();
      await waitFor(() => {
        // Version text is split across nodes: "v" + "1", use regex matching
        const versionCells = document.querySelectorAll(".text-xs.font-mono.text-neutral-600");
        const versionTexts = Array.from(versionCells)
          .map((el) => el.textContent)
          .filter((t) => t?.startsWith("v"));
        expect(versionTexts).toContain("v1");
        expect(versionTexts).toContain("v3");
      });
    });

    it("renders data table", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByTestId("data-table")).toBeInTheDocument();
      });
    });

    it("renders table headers", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("Name")).toBeInTheDocument();
        expect(screen.getByText("Size")).toBeInTheDocument();
        expect(screen.getByText("Type")).toBeInTheDocument();
        expect(screen.getByText("Ver")).toBeInTheDocument();
        expect(screen.getByText("Date")).toBeInTheDocument();
      });
    });

    it("shows download and delete action buttons for each file", async () => {
      await renderFiles();
      await waitFor(() => {
        const downloads = screen.getAllByText("download");
        expect(downloads.length).toBe(3);
        const deletes = screen.getAllByText("delete");
        expect(deletes.length).toBe(3);
      });
    });

    it("calls files.list with DID on mount", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(mockFilesList).toHaveBeenCalledWith(MOCK_DID);
      });
    });

    it("calls files.stats on mount", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(mockFilesStats).toHaveBeenCalled();
      });
    });
  });

  // ── Search / filter ───────────────────────────────────────────────────

  describe("Search filter", () => {
    it("renders filter input when files exist", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("Filter by name...")
        ).toBeInTheDocument();
      });
    });

    it("filters files by search query", async () => {
      const user = userEvent.setup();
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const searchInput = screen.getByPlaceholderText("Filter by name...");
      await user.type(searchInput, "readme");
      // Only readme.txt should remain visible
      expect(screen.getByText("readme.txt")).toBeInTheDocument();
      expect(screen.queryByText("project.zip")).not.toBeInTheDocument();
      expect(screen.queryByText("photo.png")).not.toBeInTheDocument();
    });

    it("shows no matching files state when filter has no matches", async () => {
      const user = userEvent.setup();
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const searchInput = screen.getByPlaceholderText("Filter by name...");
      await user.type(searchInput, "nonexistent");
      expect(screen.getByText("No matching files")).toBeInTheDocument();
    });

    it("shows Clear filter button when search has no results", async () => {
      const user = userEvent.setup();
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const searchInput = screen.getByPlaceholderText("Filter by name...");
      await user.type(searchInput, "nonexistent");
      expect(screen.getByText("Clear filter")).toBeInTheDocument();
    });
  });

  // ── Upload ────────────────────────────────────────────────────────────

  describe("Upload", () => {
    it("has a hidden file input", async () => {
      await renderFiles();
      const fileInputEl = document.querySelector('input[type="file"]');
      expect(fileInputEl).toBeTruthy();
      expect(fileInputEl?.className).toContain("hidden");
    });

    it("shows toast on successful upload", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("Upload File")).toBeInTheDocument();
      });
      // Simulate file input change event
      const fileInputEl = document.querySelector(
        'input[type="file"]'
      ) as HTMLInputElement;
      const file = new File(["hello"], "test.txt", {
        type: "text/plain",
      });
      Object.defineProperty(fileInputEl, "files", {
        value: [file],
      });
      fileInputEl.dispatchEvent(new Event("change", { bubbles: true }));
      await waitFor(() => {
        expect(mockFilesUpload).toHaveBeenCalled();
      });
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "File uploaded",
            variant: "success",
          })
        );
      });
    });

    it("shows error toast on upload failure", async () => {
      setupDefaults();
      mockFilesUpload.mockRejectedValue(new Error("Disk full"));
      render(<FilesPage />);
      await waitFor(() => {
        expect(screen.getByText("Upload File")).toBeInTheDocument();
      });
      const fileInputEl = document.querySelector(
        'input[type="file"]'
      ) as HTMLInputElement;
      const file = new File(["data"], "big.bin", {
        type: "application/octet-stream",
      });
      Object.defineProperty(fileInputEl, "files", {
        value: [file],
      });
      fileInputEl.dispatchEvent(new Event("change", { bubbles: true }));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Upload failed",
            variant: "error",
          })
        );
      });
    });
  });

  // ── Delete ────────────────────────────────────────────────────────────

  describe("Delete", () => {
    it("calls files.delete API", async () => {
      const user = userEvent.setup();
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const deleteButtons = screen.getAllByText("delete");
      await user.click(deleteButtons[0]);
      await waitFor(() => {
        expect(mockFilesDelete).toHaveBeenCalledWith("project.zip", MOCK_DID);
      });
    });

    it("shows toast on successful delete", async () => {
      const user = userEvent.setup();
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const deleteButtons = screen.getAllByText("delete");
      await user.click(deleteButtons[0]);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "File deleted" })
        );
      });
    });

    it("shows toast on delete failure", async () => {
      const user = userEvent.setup();
      setupDefaults();
      mockFilesDelete.mockRejectedValue(new Error("Permission denied"));
      render(<FilesPage />);
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const deleteButtons = screen.getAllByText("delete");
      await user.click(deleteButtons[0]);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Delete failed",
            variant: "error",
          })
        );
      });
    });
  });

  // ── Empty state ───────────────────────────────────────────────────────

  describe("Empty state", () => {
    it("shows empty state when no files", async () => {
      setupDefaults();
      mockFilesList.mockResolvedValue({ files: [], count: 0 });
      render(<FilesPage />);
      await waitFor(() => {
        expect(screen.getByText("No files yet")).toBeInTheDocument();
      });
    });

    it("shows Upload File CTA in empty state", async () => {
      setupDefaults();
      mockFilesList.mockResolvedValue({ files: [], count: 0 });
      render(<FilesPage />);
      await waitFor(() => {
        const emptyState = screen.getByTestId("empty-state");
        expect(
          within(emptyState).getByText("Upload File")
        ).toBeInTheDocument();
      });
    });
  });

  // ── No identity ───────────────────────────────────────────────────────

  describe("No identity", () => {
    it("shows create identity prompt when no DID", async () => {
      await renderFiles(false);
      expect(screen.getByText("Create an identity")).toBeInTheDocument();
    });

    it("does not call files.list when no DID", async () => {
      await renderFiles(false);
      expect(mockFilesList).not.toHaveBeenCalled();
    });
  });

  // ── API integration ───────────────────────────────────────────────────

  describe("API integration", () => {
    it("handles files.list failure gracefully", async () => {
      setupDefaults();
      mockFilesList.mockRejectedValue(new Error("Server down"));
      render(<FilesPage />);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Failed to load files",
            variant: "error",
          })
        );
      });
    });

    it("reloads file list after upload", async () => {
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("Upload File")).toBeInTheDocument();
      });
      const fileInputEl = document.querySelector(
        'input[type="file"]'
      ) as HTMLInputElement;
      const file = new File(["hello"], "new.txt", { type: "text/plain" });
      Object.defineProperty(fileInputEl, "files", { value: [file] });
      fileInputEl.dispatchEvent(new Event("change", { bubbles: true }));
      await waitFor(() => {
        // files.list called on mount + after upload
        expect(mockFilesList.mock.calls.length).toBeGreaterThanOrEqual(2);
      });
    });

    it("reloads file list after delete", async () => {
      const user = userEvent.setup();
      await renderFiles();
      await waitFor(() => {
        expect(screen.getByText("project.zip")).toBeInTheDocument();
      });
      const callsBefore = mockFilesList.mock.calls.length;
      const deleteButtons = screen.getAllByText("delete");
      await user.click(deleteButtons[0]);
      await waitFor(() => {
        expect(mockFilesList.mock.calls.length).toBeGreaterThan(callsBefore);
      });
    });
  });

  // ── Drag and drop ─────────────────────────────────────────────────────

  describe("Drag and drop", () => {
    it("shows drop overlay on dragenter with files", async () => {
      await renderFiles();
      const container = document.querySelector(".p-4");
      if (!container) throw new Error("Container not found");
      const dragEvent = new Event("dragenter", { bubbles: true });
      Object.defineProperty(dragEvent, "preventDefault", {
        value: vi.fn(),
      });
      Object.defineProperty(dragEvent, "dataTransfer", {
        value: { types: ["Files"] },
      });
      container.dispatchEvent(dragEvent);
      await waitFor(() => {
        expect(screen.getByText("Drop files to upload")).toBeInTheDocument();
      });
    });
  });
});
