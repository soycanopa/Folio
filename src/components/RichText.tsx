import { useEffect } from "react";
import { EditorContent, useEditor, type Editor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "tiptap-markdown";

// tiptap-markdown no tipa su storage para tiptap v3; acceso acotado.
const getMarkdown = (editor: Editor): string =>
  (
    editor.storage as {
      markdown?: { getMarkdown: () => string };
    }
  ).markdown?.getMarkdown() ?? "";

interface RichTextProps {
  value: string;
  onChange: (markdown: string) => void;
  sourceMode: boolean;
}

// El editor de Pages CMS es TipTap con salida Markdown (DESIGN.md);
// este es el mínimo propio con la misma moneda de intercambio.
export function RichText({ value, onChange, sourceMode }: RichTextProps) {
  const editor = useEditor({
    extensions: [StarterKit, Markdown.configure({ html: false })],
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
    <div className="rounded-lg bg-panel p-4 focus-within:ring-1 focus-within:ring-accent">
      <EditorContent editor={editor} />
    </div>
  );
}
