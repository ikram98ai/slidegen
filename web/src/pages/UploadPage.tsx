import React from "react";
import { NavBar } from "../components/NavBar";
import { useNavigate } from "react-router-dom";
import { useSubjectStore } from "../store/subjectStore";
import { useAddSubject } from "../hooks/useAppQueries";
import { DocType, type SubjectCreate } from "../types";
import { BookOpen, FileText } from "lucide-react";
import { FileUpload } from "../components/ui/FileUpload";
import { Button } from "../components/ui/Button";

export const UploadPage: React.FC = () => {
  const navigate = useNavigate();

  const {
    file,
    error,
    docType,
    isAnalyzing,
    setFile,
    setDocType,
    setFileBase64,
    setIsAnalyzing,
    setError,
    reset: resetUpload,
  } = useSubjectStore();

  const addSubjectMutation = useAddSubject();
  const isLoading = isAnalyzing;
  // Use passed isAnalyzing (from mutation) or store's isAnalyzing

  const onFileSelect = (selectedFile: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const result = e.target?.result as string;
      const base64 = result.split(",")[1];

      setFile(selectedFile);
      setFileBase64(base64);
      setError(null);
    };
    reader.readAsDataURL(selectedFile);
  };

  const onStartAnalysis = async () => {
    if (!file) return;

    setIsAnalyzing(true);
    setError(null);

    try {
      const newSubject: SubjectCreate = {
        title: file.name.replace(".pdf", ""),
        type: docType,
        file: file,
      };

      // Save to "Database" via Mutation
      addSubjectMutation.mutate(newSubject, {
        onSuccess: () => {
          // Reset Upload State
          resetUpload();
          navigate("/profile");
        },
        onError: () => {
          setIsAnalyzing(false);
          setError("Failed to save subject.");
        },
      });
    } catch {
      setIsAnalyzing(false);
      setError("We encountered an issue analyzing your subject.");
    }
  };
  return (
    <>
      <div className="pt-8">
        <NavBar />
      </div>
      <div className="flex flex-col items-center justify-center min-h-[calc(100vh-8rem)] p-6 animate-fade-in-up">
        <div className="text-center max-w-2xl mx-auto mb-12">
          <h1 className="text-4xl md:text-5xl font-bold text-gray-900 tracking-tight mb-6">
            Turn your reading into <br />
            <span className="text-transparent bg-clip-text bg-linear-to-r from-blue-600 to-purple-600">
              interactive mastery.
            </span>
          </h1>
          <p className="text-lg text-gray-500 font-medium leading-relaxed">
            Upload a textbook or report. Lumina analyzes the content, structures
            it, and designs beautiful presentation slides instantly with voice
            narrations.
          </p>
        </div>

        <div className="w-full max-w-3xl bg-white rounded-4xl shadow-apple-xl p-8 md:p-12 border border-white/20">
          <div className="flex justify-center mb-10">
            <div className="bg-gray-100/80 p-1.5 rounded-full flex space-x-1 shadow-inner">
              <button
                onClick={() => setDocType(DocType.BOOK)}
                className={`
                px-6 py-2 rounded-full text-sm font-semibold transition-all duration-200
                ${
                  docType === DocType.BOOK
                    ? "bg-white text-gray-900 shadow-sm"
                    : "text-gray-500 hover:text-gray-700"
                }
              `}
              >
                <div className="flex items-center space-x-2">
                  <BookOpen size={16} />
                  <span>Book</span>
                </div>
              </button>
              <button
                onClick={() => setDocType(DocType.REPORT)}
                className={`
                px-6 py-2 rounded-full text-sm font-semibold transition-all duration-200
                ${
                  docType === DocType.REPORT
                    ? "bg-white text-gray-900 shadow-sm"
                    : "text-gray-500 hover:text-gray-700"
                }
              `}
              >
                <div className="flex items-center space-x-2">
                  <FileText size={16} />
                  <span>Report</span>
                </div>
              </button>
            </div>
          </div>

          <FileUpload onFileSelect={onFileSelect} selectedFile={file} />

          <div className="mt-10 flex justify-center">
            <Button
              disabled={!file}
              isLoading={isLoading}
              onClick={onStartAnalysis}
              className="w-full sm:w-auto px-12 py-4 text-base"
            >
              {isLoading ? "Uploading..." : "Create Learning Material"}
            </Button>
          </div>
        </div>
        {error && (
          <div className="fixed bottom-6 right-6 bg-red-500 text-white px-6 py-4 rounded-xl shadow-2xl flex items-center z-50 animate-bounce-in">
            <span className="font-medium mr-2">Error:</span> {error}
            <button
              onClick={() => setError(null)}
              className="ml-4 opacity-75 hover:opacity-100"
            >
              ✕
            </button>
          </div>
        )}
      </div>{" "}
    </>
  );
};
