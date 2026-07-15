import React, { useState } from "react";
import {
  BookOpen,
  FileText,
  Clock,
  Pencil,
  Trash2,
  Eye,
  EyeOff,
  AlertCircle,
  Loader2,
  RotateCcw,
} from "lucide-react";
import type { SubjectResponse } from "../types";
import { DocType } from "../types";
import {
  useUpdateSubject,
  useDeleteSubject,
  useReprocessSubject,
} from "../hooks/useAppQueries";
import { Modal } from "./ui/Modal";
import { Button } from "./ui/Button";
import { CircularProgress } from "./ui/CircularProgress";
import { useAuthStore } from "../store/authStore";

interface SubjectCardProps {
  subject: SubjectResponse;
  onClick: (subject: SubjectResponse) => void;
}

export const SubjectCard: React.FC<SubjectCardProps> = ({
  subject,
  onClick,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editedTitle, setEditedTitle] = useState(subject.title);

  const updateSubject = useUpdateSubject();
  const deleteSubject = useDeleteSubject();
  const reprocessSubject = useReprocessSubject();
  const { user } = useAuthStore();

  const handleReprocess = (e: React.MouseEvent) => {
    e.stopPropagation();
    reprocessSubject.mutate(subject.id);
  };

  const handleUpdate = () => {
    updateSubject.mutate({ id: subject.id, data: { title: editedTitle } });
    setIsEditing(false);
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (window.confirm("Are you sure you want to delete this subject?")) {
      deleteSubject.mutate(subject.id);
    }
  };

  const toggleVisibility = (e: React.MouseEvent) => {
    e.stopPropagation();
    updateSubject.mutate({
      id: subject.id,
      data: { is_public: !subject.is_public },
    });
  };

  return (
    <>
      <div
        onClick={() => {
          if (subject.processing_status !== "processing") {
            onClick(subject);
          }
        }}
        className={`group bg-white rounded-3xl p-6 shadow-apple-sm hover:shadow-apple-xl border border-gray-100 transition-all duration-300 flex flex-col h-full ${
          subject.processing_status === "processing" ? "opacity-75 cursor-wait" : "cursor-pointer"
        }`}
      >
        <div className="flex justify-between items-start mb-6">
          <div
            className={`
            w-12 h-12 rounded-2xl flex items-center justify-center transition-colors duration-300
            ${
              subject.type === DocType.BOOK
                ? "bg-blue-100 text-blue-600 group-hover:bg-blue-600 group-hover:text-white"
                : "bg-purple-100 text-purple-600 group-hover:bg-purple-600 group-hover:text-white"
            }
          `}
          >
            {subject.type === DocType.BOOK ? (
              <BookOpen size={24} />
            ) : (
              <FileText size={24} />
            )}
          </div>
          {user && user.id === subject.user_id && (
            <div className="flex items-center space-x-2">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setIsEditing(true);
                }}
                className="p-2 rounded-full hover:bg-gray-100"
              >
                <Pencil size={16} />
              </button>
              <button
                onClick={handleDelete}
                className="p-2 rounded-full hover:bg-red-300"
              >
                <Trash2 size={16} />
              </button>
              <button
                onClick={toggleVisibility}
                className="p-2 rounded-full hover:bg-gray-100"
              >
                {subject.is_public ? <Eye size={16} /> : <EyeOff size={16} />}
              </button>
            </div>
          )}
        </div>
        <h3 className="text-xl font-bold text-gray-900 mb-2 line-clamp-2 leading-tight group-hover:text-blue-600 transition-colors">
          {subject.title}
        </h3>
        <div className="mt-auto pt-6 flex items-center justify-between text-sm text-gray-400 border-t border-gray-50">
          <div className="flex items-center space-x-2">
            <Clock size={14} />
            <span>{new Date(subject.created_at).toLocaleDateString()}</span>
          </div>
          {subject.processing_status === "processing" &&
            (subject.total_pages && subject.total_pages > 0 ? (
              <div className="flex items-center text-blue-500 font-medium">
                <CircularProgress
                  processed={subject.processed_pages ?? 0}
                  total={subject.total_pages}
                  size={38}
                />
                <span className="ml-2">
                  {(subject.processed_pages ?? 0) >= subject.total_pages
                    ? "Analyzing..."
                    : "Reading pages..."}
                </span>
              </div>
            ) : (
              <div className="flex items-center text-blue-500 font-medium">
                <Loader2 size={14} className="mr-1 animate-spin" />
                Processing...
              </div>
            ))}
          {subject.processing_status === "failed" && (
            <div className="flex items-center space-x-2">
              <div className="flex items-center text-red-500 font-medium">
                <AlertCircle size={14} className="mr-1" />
                Failed
              </div>
              {user && user.id === subject.user_id && (
                <button
                  onClick={handleReprocess}
                  disabled={reprocessSubject.isPending}
                  className="flex items-center px-2 py-1 rounded-full text-blue-600 font-medium hover:bg-blue-50 disabled:opacity-50"
                  title="Reprocess this file"
                >
                  {reprocessSubject.isPending ? (
                    <Loader2 size={14} className="mr-1 animate-spin" />
                  ) : (
                    <RotateCcw size={14} className="mr-1" />
                  )}
                  Retry
                </button>
              )}
            </div>
          )}
        </div>
      </div>
      <Modal isOpen={isEditing} onClose={() => setIsEditing(false)}>
        <div className="p-6">
          <h2 className="text-2xl font-bold mb-4">Edit Subject</h2>
          <input
            type="text"
            value={editedTitle}
            onChange={(e) => setEditedTitle(e.target.value)}
            className="w-full p-2 border rounded"
          />
          <div className="mt-4 flex justify-end space-x-2">
            <Button onClick={() => setIsEditing(false)} variant="secondary">
              Cancel
            </Button>
            <Button onClick={handleUpdate}>Save</Button>
          </div>
        </div>
      </Modal>
    </>
  );
};
