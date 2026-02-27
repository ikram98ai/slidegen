import { useState } from "react";
import { Modal } from "./ui/Modal";
import { Button } from "./ui/Button";
import { Save } from "lucide-react";
import { type ChapterCreate } from "../types";
import { chaptersApi } from "../services/api";

interface AddChapterProps {
  subjectId: string;
  isCreating: boolean;
  onSetIsCreating: (creating: boolean) => void;
}

export const AddChapter: React.FC<AddChapterProps> = ({
  subjectId,
  isCreating,
  onSetIsCreating,
}) => {
  const [title, setTitle] = useState<string>("");
  const [startPage, setStartPage] = useState<string>("0");
  const [endPage, setEndPage] = useState<string>("0");
  const [orderIndex, setOrderIndex] = useState<string>("0");
  const [isLoading, setIsLoading] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  const handleCreateChapter = async (e: React.FormEvent) => {
    e.preventDefault();

    setIsLoading(true);
    setMessage(null);

    try {
      if (!title.trim()) {
        setMessage({ type: "error", text: "Chapter title is required." });
        setIsLoading(false);
        return;
      }

      const sPage = parseInt(startPage) || 0;
      const ePage = parseInt(endPage) || 0;
      const oIndex = parseInt(orderIndex) || 0;

      const createChapter: ChapterCreate = {
        subject_id: subjectId,
        title: title.trim(),
        page_start: sPage,
        page_end: ePage,
        order_index: oIndex,
      };

      await chaptersApi.createChapter(createChapter);

      setMessage({ type: "success", text: "Chapter added successfully." });
      setTitle("");
      setStartPage("0");
      setEndPage("0");
      setOrderIndex("0");
      onSetIsCreating(false);
    } catch (error: any) {
      console.error("Create chapter failed", error);
      setMessage({ type: "error", text: error.message || "Failed to add chapter." });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Modal
      isOpen={isCreating}
      onClose={() => onSetIsCreating(false)}
      title="Add Chapter Manually"
    >
      <form onSubmit={handleCreateChapter} className="space-y-6">
        <div>
          <label className="block text-sm font-semibold text-gray-700 mb-2">
            Chapter Title
          </label>
          <div className="relative">
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g. Introduction"
              className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
              required
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-semibold text-gray-700 mb-2">
              Start Page
            </label>
            <input
              type="number"
              value={startPage}
              onChange={(e) => setStartPage(e.target.value)}
              className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            />
          </div>
          <div>
            <label className="block text-sm font-semibold text-gray-700 mb-2">
              End Page
            </label>
            <input
              type="number"
              value={endPage}
              onChange={(e) => setEndPage(e.target.value)}
              className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            />
          </div>
        </div>

        <div>
          <label className="block text-sm font-semibold text-gray-700 mb-2">
            Order Index
          </label>
          <input
            type="number"
            value={orderIndex}
            onChange={(e) => setOrderIndex(e.target.value)}
            className="w-full px-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
          />
        </div>

        {message && (
          <div
            className={`p-4 rounded-xl text-sm font-medium ${
              message.type === "success"
                ? "bg-green-50 text-green-700"
                : "bg-red-50 text-red-600"
            }`}
          >
            {message.text}
          </div>
        )}

        <div className="flex justify-end space-x-4 pt-4">
          <Button
            type="button"
            variant="ghost"
            onClick={() => onSetIsCreating(false)}
          >
            Cancel
          </Button>
          <Button type="submit" isLoading={isLoading}>
            <Save size={18} className="mr-2" />
            Save Chapter
          </Button>
        </div>
      </form>
    </Modal>
  );
};
