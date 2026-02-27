import React, { useRef, useState, useEffect } from "react";
import type { SlideResponse, SlideUpdate } from "../types";
import {
  Loader2,
  Play,
  Pause,
  FileText,
  X,
  Settings,
  Gauge,
  Volume1,
  Edit3,
  Save,
  Plus,
  Trash2,
} from "lucide-react";
import { useUpdateSlide } from "../hooks/useAppQueries";

interface SlideCardProps {
  chapterId:string,
  slide: SlideResponse;
  index: number;
  total: number;
}

export const SlideCard: React.FC<SlideCardProps> = ({
  chapterId,
  slide,
  index,
  total,
}) => {
  const [isPlaying, setIsPlaying] = useState(false);
  const [showNotes, setShowNotes] = useState(false);
  const [showAudioSettings, setShowAudioSettings] = useState(false);

  // Audio Settings State
  const [volume, setVolume] = useState(1.0);
  const [playbackRate, setPlaybackRate] = useState(1.0);

  // Edit Mode State
  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState(slide.title);
  const [editBullets, setEditBullets] = useState([...slide.points]);
  const [editExplanation, setEditExplanation] = useState(slide.explanation);

  const audioRef = useRef<HTMLAudioElement | null>(null);
  const settingsRef = useRef<HTMLDivElement | null>(null);

  const updateSlideMutation = useUpdateSlide();

  // --- AUDIO SETUP ---
  useEffect(() => {
    if (slide.voice_url) {
      if (!audioRef.current) {
        audioRef.current = new Audio(slide.voice_url);
        audioRef.current.onended = () => {
          setIsPlaying(false);
        };
      } else {
        audioRef.current.src = slide.voice_url;
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setIsPlaying(false);
      }
      audioRef.current.volume = volume;
      audioRef.current.playbackRate = playbackRate;
    } else {
      setIsPlaying(false);
    }
  }, [slide.voice_url]);

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.volume = volume;
      audioRef.current.playbackRate = playbackRate;
    }
  }, [volume, playbackRate]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        settingsRef.current &&
        !settingsRef.current.contains(event.target as Node)
      ) {
        setShowAudioSettings(false);
      }
    };
    if (showAudioSettings)
      document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showAudioSettings]);

  useEffect(() => {
    return () => {
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current = null;
      }
    };
  }, []);

  // --- HANDLERS ---
  const handleTogglePlayback = () => {
    if (!slide.voice_url) {
      // Audio generation is handled by backend now
      return;
    }
    const audio = audioRef.current;
    if (isPlaying) {
      if (audio) audio.pause();
      setIsPlaying(false);
    } else {
      if (audio) {
        audio.volume = volume;
        audio.playbackRate = playbackRate;
        audio.play().catch((e) => console.error("Audio play failed", e));
      }
      setIsPlaying(true);
    }
  };

  const handleSave = () => {
    const updatedSlide: SlideUpdate = {
      ...slide,
      title: editTitle,
      points: editBullets,
      explanation: editExplanation,
    };

    updateSlideMutation.mutate(
      {
        chapterId: chapterId,
        slideId: slide.id,
        slide: updatedSlide,
      },
      {
        onSuccess: () => {
          setIsEditing(false);
        },
      }
    );
  };

  const handleCancelEdit = () => {
    setEditTitle(slide.title);
    setEditBullets([...slide.points]);
    setEditExplanation(slide.explanation);
    setIsEditing(false);
  };

  const handleBulletChange = (idx: number, val: string) => {
    const newBullets = [...editBullets];
    newBullets[idx] = val;
    setEditBullets(newBullets);
  };

  const addBullet = () => setEditBullets([...editBullets, "New point"]);
  const removeBullet = (idx: number) =>
    setEditBullets(editBullets.filter((_, i) => i !== idx));

  return (
    <>
      <div className="w-full h-full bg-white rounded-3xl shadow-apple-lg border border-gray-100 overflow-hidden flex flex-col p-8 md:p-12 transition-transform duration-500 hover:shadow-apple-xl relative">
        {/* Header */}
        <div className="flex justify-between items-center mb-6">
          <div className="flex items-center space-x-3">
            <div className="h-1.5 w-16 bg-linear-to-r from-blue-400 to-purple-400 rounded-full"></div>
            <span className="text-xs font-semibold text-gray-400 uppercase tracking-widest">
              Slide {index + 1} / {total}
            </span>
          </div>

          {/* Edit Controls */}
          <div className="flex items-center space-x-2">
            {!isEditing ? (
              <button
                onClick={() => setIsEditing(true)}
                className="p-2 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-full transition-all"
                title="Edit Slide"
              >
                <Edit3 size={18} />
              </button>
            ) : (
              <>
                <button
                  onClick={handleCancelEdit}
                  className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-full transition-all"
                  title="Cancel"
                >
                  <X size={18} />
                </button>
                <button
                  onClick={handleSave}
                  disabled={updateSlideMutation.isPending}
                  className="p-2 text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-full transition-all"
                  title="Save"
                >
                  {updateSlideMutation.isPending ? (
                    <Loader2 size={18} className="animate-spin" />
                  ) : (
                    <Save size={18} />
                  )}
                </button>
              </>
            )}
          </div>
        </div>

        <div className="flex justify-between items-start gap-6 mb-8 relative z-10">
          {isEditing ? (
            <input
              type="text"
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              className="text-3xl md:text-4xl font-bold text-gray-900 leading-tight tracking-tight w-full bg-gray-50 border-b-2 border-blue-500 focus:outline-none p-2 rounded-t-lg"
            />
          ) : (
            <h2 className="text-3xl md:text-4xl font-bold text-gray-900 leading-tight tracking-tight max-w-3xl">
              {slide.title}
            </h2>
          )}

          {/* Controls - Hide during edit to reduce clutter? Or keep? Let's hide audio controls during edit */}
          {!isEditing && (
            <div className="flex items-center space-x-2 shrink-0 relative">
              <button
                onClick={() => setShowNotes(true)}
                className="w-10 h-10 rounded-full bg-gray-50 text-gray-600 hover:bg-gray-100 flex items-center justify-center transition-all"
                title="Read Script"
              >
                <FileText size={18} />
              </button>

              {slide.voice_url && (
                <div ref={settingsRef} className="relative">
                  <button
                    onClick={() => setShowAudioSettings(!showAudioSettings)}
                    className={`w-10 h-10 rounded-full flex items-center justify-center transition-all ${
                      showAudioSettings
                        ? "bg-gray-200 text-gray-900"
                        : "bg-gray-50 text-gray-600 hover:bg-gray-100"
                    }`}
                    title="Audio Settings"
                  >
                    <Settings size={18} />
                  </button>
                  {showAudioSettings && (
                    <div className="absolute right-0 top-full mt-3 p-5 bg-white/90 backdrop-blur-xl rounded-2xl shadow-apple-xl border border-white/20 w-72 z-30 animate-fade-in origin-top-right">
                      <div className="mb-5">
                        <div className="flex items-center justify-between mb-2">
                          <span className="text-xs font-semibold text-gray-500 uppercase tracking-wider flex items-center">
                            <Volume1 size={14} className="mr-1.5" /> Volume
                          </span>
                          <span className="text-xs font-bold text-gray-900">
                            {Math.round(volume * 100)}%
                          </span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.05"
                          value={volume}
                          onChange={(e) =>
                            setVolume(parseFloat(e.target.value))
                          }
                          className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600"
                        />
                      </div>
                      <div>
                        <div className="flex items-center justify-between mb-3">
                          <span className="text-xs font-semibold text-gray-500 uppercase tracking-wider flex items-center">
                            <Gauge size={14} className="mr-1.5" /> Speed
                          </span>
                          <span className="text-xs font-bold text-gray-900">
                            {playbackRate}x
                          </span>
                        </div>
                        <div className="flex justify-between bg-gray-100/50 p-1 rounded-lg">
                          {[0.75, 1, 1.25, 1.5, 2].map((rate) => (
                            <button
                              key={rate}
                              onClick={() => setPlaybackRate(rate)}
                              className={`px-2 py-1.5 rounded-md text-xs font-semibold transition-all duration-200 flex-1 ${
                                playbackRate === rate
                                  ? "bg-white text-blue-600 shadow-sm scale-105"
                                  : "text-gray-500 hover:text-gray-700 hover:bg-white/50"
                              }`}
                            >
                              {rate}x
                            </button>
                          ))}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {slide.voice_url && (
                <button
                  onClick={handleTogglePlayback}
                  className={`w-12 h-12 rounded-full flex items-center justify-center transition-all duration-300 shadow-md ml-1 ${
                    isPlaying
                      ? "bg-red-500 text-white hover:bg-red-600"
                      : "bg-black text-white hover:bg-gray-800"
                  }`}
                >
                  {isPlaying ? (
                    <Pause size={20} fill="currentColor" />
                  ) : (
                    <Play size={20} fill="currentColor" className="ml-1" />
                  )}
                </button>
              )}
            </div>
          )}
        </div>

        {/* Content Area */}
        <div className="grow flex flex-col justify-center">
          <div className="space-y-6">
            <ul className="space-y-6">
              {isEditing ? (
                <>
                  {editBullets.map((bullet, i) => (
                    <li key={i} className="flex items-center group">
                      <span className="inline-block w-2.5 h-2.5 mr-6 bg-blue-500 rounded-full shrink-0"></span>
                      <input
                        type="text"
                        value={bullet}
                        onChange={(e) => handleBulletChange(i, e.target.value)}
                        className="flex-1 text-xl text-gray-700 font-medium bg-gray-50 border-b border-gray-300 focus:border-blue-500 focus:bg-white p-2 rounded outline-none"
                      />
                      <button
                        onClick={() => removeBullet(i)}
                        className="ml-2 text-gray-300 hover:text-red-500 transition-colors"
                      >
                        <Trash2 size={20} />
                      </button>
                    </li>
                  ))}
                  <button
                    onClick={addBullet}
                    className="flex items-center text-blue-600 font-semibold hover:bg-blue-50 p-2 rounded-lg transition-colors"
                  >
                    <Plus size={18} className="mr-2" /> Add Point
                  </button>
                </>
              ) : (
                slide.points.map((point, i) => (
                  <li key={i} className="flex items-start group">
                    <span className="inline-block w-2.5 h-2.5 mt-3 mr-6 bg-blue-500 rounded-full group-hover:scale-125 transition-transform duration-300 shrink-0"></span>
                    <span className="text-xl md:text-2xl text-gray-700 leading-relaxed font-medium">
                      {point}
                    </span>
                  </li>
                ))
              )}
            </ul>
          </div>
        </div>
      </div>

      {/* Notes Modal */}
      {showNotes && (
        <div className="fixed inset-0 z-100 flex items-center justify-center px-4">
          <div
            className="absolute inset-0 bg-black/20 backdrop-blur-sm transition-opacity"
            onClick={() => setShowNotes(false)}
          />
          <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-fade-in-up">
            <div className="flex justify-between items-center p-6 border-b border-gray-100 bg-gray-50/50">
              <h3 className="text-lg font-semibold text-gray-900">
                {isEditing ? "Edit Notes" : "Instructor Notes"}
              </h3>
              <button
                onClick={() => setShowNotes(false)}
                className="text-gray-400 hover:text-gray-600 transition-colors p-1 rounded-full hover:bg-gray-100"
              >
                <X size={20} />
              </button>
            </div>
            <div className="p-6 md:p-8">
              {isEditing ? (
                <textarea
                  value={editExplanation}
                  onChange={(e) => setEditExplanation(e.target.value)}
                  className="w-full h-40 p-4 bg-gray-50 rounded-xl text-gray-700 text-lg leading-relaxed focus:ring-2 focus:ring-blue-500 outline-none resize-none"
                  placeholder="Enter slide explanation script..."
                />
              ) : (
                <p className="text-gray-700 text-lg leading-relaxed italic">
                  "{slide.explanation}"
                </p>
              )}
            </div>
            {slide.voice_url && !isEditing && (
              <div className="p-4 bg-gray-50 border-t border-gray-100 flex justify-end">
                <button
                  onClick={() => {
                    setShowNotes(false);
                    if (!isPlaying) handleTogglePlayback();
                  }}
                  className="text-sm font-medium text-blue-600 hover:text-blue-700 px-4 py-2 rounded-lg hover:bg-blue-50 transition-colors"
                >
                  Close & Play Audio
                </button>
              </div>
            )}
            {isEditing && (
              <div className="p-4 bg-gray-50 border-t border-gray-100 flex justify-end">
                <span className="text-xs text-gray-400 mr-auto flex items-center">
                  Changes saved when you click Save on the slide card.
                </span>
                <button
                  onClick={() => setShowNotes(false)}
                  className="text-sm font-medium text-blue-600 px-4 py-2"
                >
                  Done
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
};
