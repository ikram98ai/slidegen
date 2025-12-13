import React, { useState, useEffect } from 'react';
import { useAuthStore } from '../store/authStore';
import { usersApi } from '../services/api';
import { Button } from './Button';
import { User, Mail, Save, Upload } from 'lucide-react';
import { useUserSubjects } from '../hooks/useAppQueries';
import { DocumentCard } from './DocumentCard';
import type { StoredDocument } from '../types';

interface ProfileProps{
onDocumentClick: (doc: StoredDocument) => void
}
export const ProfilePage: React.FC<ProfileProps> = ({onDocumentClick}) => {
  const { user, updateUser } = useAuthStore();
  const [name, setName] = useState(user?.full_name || '');
  const [email, setEmail] = useState(user?.email || '');
  const [isEditing, setIsEditing] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error', text: string } | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string | null>(null);

  const { data: subjects, isLoading: subjectsLoading } = useUserSubjects(user?.id);

  useEffect(() => {
    if (user) {
      setName(user.full_name);
      setEmail(user.email);
    }
  }, [user]);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setSelectedFile(e.target.files[0]);
      setPreview(URL.createObjectURL(e.target.files[0]));
    }
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user) return;
    
    setIsLoading(true);
    setMessage(null);
    
    try {
      const formData = new FormData();
      const updateData: Record<string, string> = {};
      
      if (name !== user.full_name) updateData.full_name = name;
      if (email !== user.email) updateData.email = email;
      
      if (selectedFile) {
        formData.append('dp', selectedFile);
      }
      
      if (Object.keys(updateData).length === 0 && !selectedFile) {
          setIsLoading(false);
          setIsEditing(false);
          return;
      }

      formData.append('user_update', JSON.stringify(updateData));

      const updatedUser = await usersApi.updateUser(formData);
      
      // Update the store with the new user data
      updateUser(updatedUser);
      
      setMessage({ type: 'success', text: 'Profile updated successfully.' });
      setIsEditing(false);
      setSelectedFile(null);
      setPreview(null);
      
    } catch (error) {
      console.error("Update failed", error);
      setMessage({ type: 'error', text: 'Failed to update profile.' });
    } finally {
      setIsLoading(false);
    }
  };


  if (!user) return null;

  return (
    <div className="max-w-4xl mx-auto p-6 animate-fade-in">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
        <div className="md:col-span-1">
          <div className="bg-white rounded-4xl shadow-apple-xl p-8 border border-gray-100">
            <div className="flex flex-col items-center">
              <div className="relative">
                <img
                  src={preview || user.dp || `https://ui-avatars.com/api/?name=${user.full_name}&background=random`}
                  alt="Profile"
                  className="w-24 h-24 rounded-full object-cover"
                />
                {isEditing && (
                  <label htmlFor="dp-upload" className="absolute bottom-0 right-0 bg-blue-500 text-white rounded-full p-2 cursor-pointer hover:bg-blue-600 transition-colors">
                    <Upload size={16} />
                    <input id="dp-upload" type="file" className="hidden" onChange={handleFileChange} accept="image/png, image/jpeg" />
                  </label>
                )}
              </div>
              <h2 className="text-xl font-semibold text-gray-900 mt-4">{user.full_name}</h2>
              <p className="text-gray-500">{user.email}</p>
            </div>
          </div>
        </div>

        <div className="md:col-span-2">
          <div className="bg-white rounded-4xl shadow-apple-xl p-8 md:p-12 border border-gray-100">
            <h1 className="text-3xl font-bold text-gray-900 mb-8">Edit Profile</h1>
            <form onSubmit={handleUpdate} className="space-y-6">
              <div>
                <label className="block text-sm font-semibold text-gray-700 mb-2">Full Name</label>
                <div className="relative">
                  <User className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" size={20} />
                  <input 
                    type="text" 
                    value={name}
                    onChange={e => setName(e.target.value)}
                    disabled={!isEditing}
                    className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none disabled:opacity-60"
                  />
                </div>
              </div>

              <div>
                <label className="block text-sm font-semibold text-gray-700 mb-2">Email Address</label>
                <div className="relative">
                  <Mail className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" size={20} />
                  <input 
                    type="email" 
                    value={email}
                    onChange={e => setEmail(e.target.value)}
                    disabled={!isEditing}
                    className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none disabled:opacity-60"
                  />
                </div>
              </div>

              {message && (
                <div className={`p-4 rounded-xl text-sm font-medium ${message.type === 'success' ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-600'}`}>
                  {message.text}
                </div>
              )}

              <div className="flex justify-end space-x-4 pt-4">
                {isEditing ? (
                  <>
                    <Button type="button" variant="ghost" onClick={() => setIsEditing(false)}>Cancel</Button>
                    <Button type="submit" isLoading={isLoading}>
                      <Save size={18} className="mr-2" />
                      Save Changes
                    </Button>
                  </>
                ) : (
                  <Button type="button" onClick={() => setIsEditing(true)}>
                    Edit Profile
                  </Button>
                )}
              </div>
            </form>
          </div>
        </div>
      </div>
      
      <div className="mt-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">Your Uploads</h2>
        {subjectsLoading ? (
          <p>Loading subjects...</p>
        ) : subjects && subjects.length > 0 ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
            {subjects.map(doc => (
              <DocumentCard
                key={doc.id}
                doc={doc}
                onClick={() => onDocumentClick(doc)}
              />
            ))}
          </div>
        ) : (
          <p>You haven't uploaded any documents yet.</p>
        )}
      </div>
    </div>
  );
};
