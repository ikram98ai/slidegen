import { useState } from "react";
import { Modal } from "./ui/Modal";
import { Button } from "./ui/Button";
import { Save } from "lucide-react";
import { type ChapterCreate } from "../types";
import { chaptersApi } from "../services/api";

interface AddChapterProps {
  subjectId: number;

  isCreating: boolean;
  onSetIsCreating: (creating: boolean) => void;
}

export const AddChpater: React.FC<AddChapterProps> = ({
  subjectId,
  isCreating,
  onSetIsCreating,
}) => {
  const [title, setTitle] = useState<string>("");
  const [startPage, setStartPage] = useState(0);
  const [endPage, setEndPage] = useState(0);
  const [orderIndex, setOrderIndex] = useState(0);
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
      if (title && title === "" && startPage === 0 && endPage == 0) return;

      const createChapter: ChapterCreate = {
        subject_id: subjectId,
        title: title,
        page_start: startPage,
        page_end: endPage,
        order_index: orderIndex,
      };

      await chaptersApi.createChapter(createChapter);

      setMessage({ type: "success", text: "Chapter added successfully." });
      onSetIsCreating(false);
    } catch (error) {
      console.error("Update failed", error);
      setMessage({ type: "error", text: "Failed to add chapter." });
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
            {/* <User
                  className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400"
                  size={20}
                /> */}
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            />
          </div>
        </div>

        <div>
          <label className="block text-sm font-semibold text-gray-700 mb-2">
            Start Page
          </label>
          <div className="relative">
            {/* <Mail
                  className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400"
                  size={20}
                /> */}
            <input
              type="number"
              value={startPage}
              onChange={(e) => setStartPage(parseInt(e.target.value))}
              className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            />
          </div>
        </div>
        <div>
          <label className="block text-sm font-semibold text-gray-700 mb-2">
            End Page
          </label>
          <div className="relative">
            {/* <Mail
                  className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400"
                  size={20}
                /> */}
            <input
              type="text"
              value={endPage}
              onChange={(e) => setEndPage(parseInt(e.target.value))}
              className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            />
          </div>
        </div>

        <div>
          <label className="block text-sm font-semibold text-gray-700 mb-2">
            Order Index
          </label>
          <div className="relative">
            {/* <Mail
                  className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400"
                  size={20}
                /> */}
            <input
              type="text"
              value={orderIndex}
              onChange={(e) => setOrderIndex(parseInt(e.target.value))}
              className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            />
          </div>
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
            Save Changes
          </Button>
        </div>
      </form>
    </Modal>
  );
};
