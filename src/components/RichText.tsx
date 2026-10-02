import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Bold,
  Code,
  Heading1,
  Heading2,
  Heading3,
  ImagePlus,
  Italic,
  Link2,
  List,
  ListOrdered,
  Minus,
  Quote,
  Redo2,
  SquareCode,
  Strikethrough,
  Undo2,
} from "lucide-react";
import { EditorContent, useEditor, type Editor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Image from "@tiptap/extension-image";
import { Markdown } from "tiptap-markdown";
import type { MediaRef } from "../types";
import { MediaPickerDialog } from "./Media";

// tiptap-markdown no tipa su storage para tiptap v3; acceso acotado.
const getMarkdown = (editor: Editor): string =>
  (
    editor.storage as {
      markdown?: { getMarkdown: () => string };
    }
  ).markdown?.getMarkdown() ?? "";

// El nodo image guarda la ruta pública del sitio (p.ej. /images/a.png)
// en el Markdown, pero el DOM del editor la previsualiza vía asset
// protocol, igual que el resto de miniaturas de Folio.
function makeSiteImage(root: string, mediaInput: string | null) {
  return Image.extend({
    renderHTML({ node, HTMLAttributes }) {
      const src = String(node.attrs.src ?? "");
      const name = src.split("/").pop();
      const display =
        name && mediaInput
          ? convertFileSrc(`${root}/${mediaInput}/${name}`)
          : src;
      return ["img", { ...HTMLAttributes, src: display }];
    },
  });
}

interface RichTextProps {
  value: string;
  onChange: (markdown: string) => void;
  sourceMode: boolean;
  /** Media para el botón de imagen del editor (opcional). */
  media?: {
    root: string;
    mediaInput: string;
    items: MediaRef[];
    onUpload: () => Promise<MediaRef | null>;
  };
}

// El editor de Pages CMS es TipTap con salida Markdown (DESIGN.md);
// este replica su set de controles (su components/ui/editor) sobre el
// StarterKit, sin su cliente HTTP.
export function RichText({ value, onChange, sourceMode, media }: RichTextProps) {
  const editor = useEditor({
    extensions: [
      StarterKit,
      makeSiteImage(media?.root ?? "", media?.mediaInput ?? null),
      Markdown.configure({ html: false }),
    ],
    content: value,
    onUpdate: ({ editor }) => {
      onChange(getMarkdown(editor));
    },
  });

  // Resincroniza cuando el valor cambia por fuera (cargar otra entrada).
  useEffect(() => {
    if (editor && !editor.isDestroyed && getMarkdown(editor) !== value) {
      editor.commands.setContent(value, { emitUpdate: false });
    }
  }, [value, editor]);

  if (sourceMode) {
    return (
      <textarea
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
        spellCheck={false}
        className="min-h-[240px] w-full resize-y rounded-lg bg-panel p-4 font-mono text-sm outline-none focus:ring-1 focus:ring-accent"
      />
    );
  }

  return (
    <RichToolbar editor={editor} media={media}>
      <EditorContent editor={editor} className="px-4 pb-4 pt-1" />
    </RichToolbar>
  );
}

function ToolButton({
  label,
  active,
  disabled,
  onClick,
  children,
}: {
  label: string;
  active?: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
      className={`rounded-md p-1.5 disabled:opacity-40 ${
        active
          ? "bg-raised text-ink"
          : "text-ink-dim hover:bg-raised/60 hover:text-ink"
      }`}
    >
      {children}
    </button>
  );
}

const Divider = () => <span className="mx-1 h-5 w-px bg-line" />;

function RichToolbar({
  editor,
  media,
  children,
}: {
  editor: Editor | null;
  media?: RichTextProps["media"];
  children: React.ReactNode;
}) {
  // Re-render en cada transacción para reflejar los estados activos.
  const [, bump] = useState(0);
  useEffect(() => {
    if (!editor) return;
    const handler = () => bump((b) => b + 1);
    editor.on("transaction", handler);
    return () => {
      editor.off("transaction", handler);
    };
  }, [editor]);

  const [linkOpen, setLinkOpen] = useState(false);
  const [linkUrl, setLinkUrl] = useState("");
  const [pickerOpen, setPickerOpen] = useState(false);

  if (!editor) {
    return (
      <div className="rounded-lg bg-panel p-4">
        <div className="min-h-[240px]" />
      </div>
    );
  }

  const chain = () => editor.chain().focus();

  const applyLink = () => {
    const href = linkUrl.trim();
    if (!href) {
      chain().extendMarkRange("link").unsetLink().run();
    } else {
      chain().extendMarkRange("link").setLink({ href }).run();
    }
    setLinkOpen(false);
    setLinkUrl("");
  };

  return (
    <div className="rounded-lg bg-panel focus-within:ring-1 focus-within:ring-accent">
      <div className="flex flex-wrap items-center gap-0.5 border-b border-line px-2 py-1.5">
        <ToolButton
          label="Bold"
          active={editor.isActive("bold")}
          onClick={() => chain().toggleBold().run()}
        >
          <Bold size={14} />
        </ToolButton>
        <ToolButton
          label="Italic"
          active={editor.isActive("italic")}
          onClick={() => chain().toggleItalic().run()}
        >
          <Italic size={14} />
        </ToolButton>
        <ToolButton
          label="Strikethrough"
          active={editor.isActive("strike")}
          onClick={() => chain().toggleStrike().run()}
        >
          <Strikethrough size={14} />
        </ToolButton>
        <ToolButton
          label="Inline code"
          active={editor.isActive("code")}
          onClick={() => chain().toggleCode().run()}
        >
          <Code size={14} />
        </ToolButton>

        <Divider />
        <ToolButton
          label="Heading 1"
          active={editor.isActive("heading", { level: 1 })}
          onClick={() => chain().toggleHeading({ level: 1 }).run()}
        >
          <Heading1 size={14} />
        </ToolButton>
        <ToolButton
          label="Heading 2"
          active={editor.isActive("heading", { level: 2 })}
          onClick={() => chain().toggleHeading({ level: 2 }).run()}
        >
          <Heading2 size={14} />
        </ToolButton>
        <ToolButton
          label="Heading 3"
          active={editor.isActive("heading", { level: 3 })}
          onClick={() => chain().toggleHeading({ level: 3 }).run()}
        >
          <Heading3 size={14} />
        </ToolButton>

        <Divider />
        <ToolButton
          label="Bulleted list"
          active={editor.isActive("bulletList")}
          onClick={() => chain().toggleBulletList().run()}
        >
          <List size={14} />
        </ToolButton>
        <ToolButton
          label="Numbered list"
          active={editor.isActive("orderedList")}
          onClick={() => chain().toggleOrderedList().run()}
        >
          <ListOrdered size={14} />
        </ToolButton>

        <Divider />
        <ToolButton
          label="Quote"
          active={editor.isActive("blockquote")}
          onClick={() => chain().toggleBlockquote().run()}
        >
          <Quote size={14} />
        </ToolButton>
        <ToolButton
          label="Code block"
          active={editor.isActive("codeBlock")}
          onClick={() => chain().toggleCodeBlock().run()}
        >
          <SquareCode size={14} />
        </ToolButton>
        <ToolButton
          label="Horizontal rule"
          onClick={() => chain().setHorizontalRule().run()}
        >
          <Minus size={14} />
        </ToolButton>

        <Divider />
        <div className="relative">
          <ToolButton
            label={editor.isActive("link") ? "Remove link" : "Add link"}
            active={editor.isActive("link")}
            onClick={() => {
              if (editor.isActive("link")) {
                chain().extendMarkRange("link").unsetLink().run();
              } else {
                setLinkUrl("");
                setLinkOpen(true);
              }
            }}
          >
            <Link2 size={14} />
          </ToolButton>
          {linkOpen && (
            <>
              <div className="fixed inset-0 z-10" onClick={() => setLinkOpen(false)} />
              <div className="absolute left-0 top-full z-20 mt-1 flex w-64 items-center gap-1.5 rounded-lg border border-line bg-panel p-2 shadow-xl">
                <input
                  autoFocus
                  value={linkUrl}
                  onChange={(e) => setLinkUrl(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") applyLink();
                    if (e.key === "Escape") setLinkOpen(false);
                  }}
                  placeholder="https://…"
                  className="min-w-0 flex-1 rounded-md bg-canvas px-2 py-1 text-sm outline-none focus:ring-1 focus:ring-accent"
                />
                <button
                  onClick={applyLink}
                  className="rounded-md bg-accent px-2 py-1 text-xs font-medium text-accent-ink"
                >
                  Apply
                </button>
              </div>
            </>
          )}
        </div>
        {media && (
          <ToolButton
            label="Image"
            onClick={() => setPickerOpen(true)}
          >
            <ImagePlus size={14} />
          </ToolButton>
        )}

        <Divider />
        <ToolButton
          label="Undo"
          disabled={!editor.can().undo()}
          onClick={() => chain().undo().run()}
        >
          <Undo2 size={14} />
        </ToolButton>
        <ToolButton
          label="Redo"
          disabled={!editor.can().redo()}
          onClick={() => chain().redo().run()}
        >
          <Redo2 size={14} />
        </ToolButton>
      </div>

      <div
        onClick={() => {
          setLinkOpen(false);
        }}
      >
        {children}
      </div>

      {media && pickerOpen && (
        <MediaPickerDialog
          root={media.root}
          items={media.items}
          onUpload={() => media.onUpload()}
          onPick={(m) => {
            setPickerOpen(false);
            chain().setImage({ src: m.public_path, alt: m.name }).run();
          }}
          onClose={() => setPickerOpen(false)}
        />
      )}
    </div>
  );
}
