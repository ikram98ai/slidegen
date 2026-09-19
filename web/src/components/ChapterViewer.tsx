import React, { useEffect, useRef, useState } from "react";
import { BookOpen, Loader2, RefreshCw } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useChapterEmbed } from "../hooks/useAppQueries";
import { chaptersApi } from "../services/api";
import { Button } from "./ui/Button";
import { CircularProgress } from "./ui/CircularProgress";
import type { ChapterResponse } from "../types";

interface CitationEvent {
  pdf_page?: number;
  printed_page?: number | null;
  paragraph_id?: string;
  quote?: string;
}

interface ChapterViewerProps {
  subjectId: string;
  chapter: ChapterResponse;
}

export const ChapterViewer: React.FC<ChapterViewerProps> = ({
  subjectId,
  chapter,
}) => {
  const queryClient = useQueryClient();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [height, setHeight] = useState(720);
  const [citation, setCitation] = useState<CitationEvent | null>(null);
  const isProcessing = chapter.processing_status === "processing";
  const hasPackage = Boolean(chapter.package_key);

  const { data: embed, isError, isFetching, refetch } = useChapterEmbed(
    subjectId,
    chapter.id,
    hasPackage && !isProcessing
  );

  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      const data = event.data;
      if (!data || data.source !== "slidegen") return;
      if (data.type === "slidegen:resize" && typeof data.height === "number") {
        setHeight(Math.max(480, Math.ceil(data.height)));
      }
      if (data.type === "slidegen:citation") {
        setCitation({
          pdf_page: data.pdf_page,
          printed_page: data.printed_page,
          paragraph_id: data.paragraph_id,
          quote: data.quote,
        });
      }
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, []);

  const generateChapter = async () => {
    try {
      await chaptersApi.generateChapter(subjectId, chapter.id);
      queryClient.invalidateQueries({ queryKey: ["subject", subjectId] });
    } catch (err) {
      alert(
        err instanceof Error ? err.message : "Failed to start chapter generation."
      );
    }
  };

  if (isProcessing) {
    const total = chapter.total_slides ?? 0;
    const processed = chapter.processed_slides ?? 0;
    return (
      <div className="flex flex-col items-center justify-center flex-1 bg-white rounded-3xl shadow-sm border border-gray-100 min-h-[400px]">
        {total > 0 ? (
          <>
            <CircularProgress processed={processed} total={total} size={96} />
            <p className="text-gray-500 mt-6 font-medium">
              Building scenes with narration — {processed} of {total} ready
            </p>
          </>
        ) : (
          <>
            <Loader2 className="w-16 h-16 text-blue-500 mb-4 animate-spin" />
            <p className="text-gray-500 mb-6 font-medium">
              Reading the chapter and planning interactive scenes...
            </p>
          </>
        )}
      </div>
    );
  }

  if (!hasPackage) {
    return (
      <div className="flex flex-col items-center justify-center flex-1 bg-white rounded-3xl shadow-sm border border-gray-100 min-h-[400px]">
        <BookOpen className="w-16 h-16 text-gray-300 mb-4" />
        <p className="text-gray-500 mb-6">
          Ready to turn this section into an interactive chapter.
        </p>
        <Button onClick={generateChapter}>Generate Chapter</Button>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center justify-center flex-1 bg-white rounded-3xl shadow-sm border border-gray-100 min-h-[400px]">
        <p className="text-gray-500 mb-6">Could not load the chapter package.</p>
        <Button onClick={() => refetch()}>Try again</Button>
      </div>
    );
  }

  if (!embed?.embed_url || isFetching) {
    return (
      <div className="flex items-center justify-center flex-1 bg-white rounded-3xl shadow-sm border border-gray-100 min-h-[400px] text-gray-400">
        <Loader2 size={32} className="animate-spin mr-3 text-blue-500" />
        Loading chapter...
      </div>
    );
  }

  const pageLabel =
    citation?.printed_page != null
      ? `p. ${citation.printed_page}`
      : citation?.pdf_page != null
        ? `PDF p. ${citation.pdf_page}`
        : null;

  return (
    <div className="flex flex-col gap-4 flex-1">
      <iframe
        ref={iframeRef}
        title={chapter.title}
        src={embed.embed_url}
        sandbox="allow-scripts"
        className="w-full bg-white rounded-3xl shadow-sm border border-gray-100"
        style={{ height, minHeight: 480 }}
      />
      {citation && (
        <div className="rounded-2xl border border-blue-100 bg-blue-50 px-5 py-4 text-sm text-blue-900">
          <div className="font-semibold mb-1">
            Source{pageLabel ? ` · ${pageLabel}` : ""}
            {citation.paragraph_id ? ` · ${citation.paragraph_id}` : ""}
          </div>
          <p className="text-blue-800/80">{citation.quote}</p>
        </div>
      )}
      <div className="flex justify-end">
        <button
          type="button"
          onClick={generateChapter}
          className="inline-flex items-center gap-2 text-sm text-gray-500 hover:text-blue-600"
        >
          <RefreshCw size={14} />
          Regenerate chapter
        </button>
      </div>
    </div>
  );
};
