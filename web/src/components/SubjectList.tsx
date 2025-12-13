import React from "react";
import { Layout } from "lucide-react";
import type { SubjectResponse } from "../types";
import { SubjectCard } from "./SubjectCard";

interface SubjectListProps {
  subjects: SubjectResponse[] | undefined;
  isLoading: boolean;
  onSubjectClick: (subject: SubjectResponse) => void;
}

export const SubjectList: React.FC<SubjectListProps> = ({
  subjects,
  isLoading,
  onSubjectClick,
}) => {
  const emptyMsg = "No public subjects found.";

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {!subjects || subjects.length === 0 ? (
        <div className="text-center py-20">
          <div className="bg-gray-100 w-16 h-16 rounded-full flex items-center justify-center mx-auto mb-4">
            <Layout className="text-gray-400" />
          </div>
          <h3 className="text-lg font-medium text-gray-900">
            {isLoading ? "Loading library..." : emptyMsg}
          </h3>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-8 animate-fade-in">
          {subjects.map((subject) => (
            <SubjectCard
              key={subject.id}
              subject={subject}
              onClick={onSubjectClick}
            />
          ))}
        </div>
      )}
    </div>
  );
};
