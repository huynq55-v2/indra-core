import React, { useState, useEffect } from 'react';
import axios from 'axios';
import GraphViewer from './components/GraphViewer'; // Import component GraphViewer

function App() {
  const [isLogin, setIsLogin] = useState(true);
  const [form, setForm] = useState({ username: '', password: '', invite_code: '' });
  const [userData, setUserData] = useState<any>(null); // Lưu thông tin sau login

  // Khôi phục phiên đăng nhập từ LocalStorage
  useEffect(() => {
    const savedSession = localStorage.getItem('indra_session');
    if (savedSession) {
      setUserData(JSON.parse(savedSession));
    }
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const endpoint = isLogin ? "/login" : "/register";
    try {
      const { data } = await axios.post("http://localhost:3000/api/auth" + endpoint, form);
      setUserData(data);
      // Lưu vào LocalStorage
      localStorage.setItem('indra_session', JSON.stringify(data));
    } catch (err: any) {
      console.error("Lỗi kết nối:", err);
      if (err.response) {
        alert("Lỗi: " + (err.response.data?.message || err.response.data || "Server từ chối yêu cầu"));
      } else if (err.request) {
        alert("Lỗi: Không nhận được phản hồi từ Server. Kiểm tra xem Backend đã chạy chưa hoặc lỗi CORS.");
      } else {
        alert("Lỗi thiết lập yêu cầu: " + err.message);
      }
    }
  };

  const handleLogout = () => {
    localStorage.removeItem('indra_session');
    setUserData(null);
    setForm({ username: '', password: '', invite_code: '' });
  };

  // Nếu đã đăng nhập thành công, hiện màn hình Đồ thị
  if (userData) {
    return (
      <div className="min-h-screen bg-slate-950 p-8 text-white font-sans">
        <div className="max-w-6xl mx-auto">
          <div className="flex justify-between items-start mb-8">
            <h1 className="text-3xl font-bold text-cyan-400 font-mono">IndraCore<span className="text-teal-500">_Network</span></h1>
            <div className="text-right flex flex-col items-end gap-2">
              <div className="flex items-center gap-3">
                <span className="text-slate-400 text-sm">User ID: <span className="text-slate-200 font-mono">{userData.user_id}</span></span>
                <span className={`text-xs px-2 py-1 rounded font-bold bg-slate-800 text-emerald-400 border border-slate-700`}>
                  ● CONNECTED
                </span>
              </div>
              <button 
                onClick={handleLogout}
                className="text-xs text-rose-400 hover:text-rose-300 hover:bg-rose-950/50 px-3 py-1.5 rounded transition-colors border border-transparent hover:border-rose-900/50 mt-1"
              >
                Ngắt kết nối (Logout)
              </button>
            </div>
          </div>
          
          <div className="grid grid-cols-[1fr_300px] gap-6 items-start">
            <div className="bg-slate-900 border border-slate-800 rounded-lg overflow-hidden h-full min-h-[500px]">
              <GraphViewer userId={userData.user_id} />
            </div>
            
            <div className="flex flex-col gap-6">
              <div className="bg-slate-900 border border-slate-800 p-5 rounded-lg">
                <div className="flex justify-between items-center mb-4">
                  <h3 className="text-cyan-500 font-bold">Quản lý Mã Mời</h3>
                  <button 
                    onClick={async () => {
                      try {
                        const { data } = await axios.post("http://localhost:3000/api/auth/invite/generate", { user_id: userData.user_id });
                        const updated = {...userData, invite_codes: [...(userData.invite_codes || []), data]};
                        setUserData(updated);
                        localStorage.setItem('indra_session', JSON.stringify(updated));
                      } catch (err) {
                        alert("Không thể tạo mã mời mới lúc này!");
                      }
                    }}
                    className="text-xs bg-cyan-600 hover:bg-cyan-500 text-white px-3 py-1.5 rounded transition-colors"
                  >
                    + Tạo mã mời
                  </button>
                </div>
                
                {(!userData.invite_codes || userData.invite_codes.length === 0) ? (
                  <p className="text-slate-500 text-sm italic">Bạn chưa tạo mã mời nào.</p>
                ) : (
                  <ul className="space-y-2">
                    {userData.invite_codes.map((ic: any, index: number) => (
                      <li key={index} className="flex justify-between items-center p-2 bg-slate-950 rounded border border-slate-800">
                        <span className="font-mono text-cyan-400 text-sm font-bold">{ic.code}</span>
                        <span className={`text-xs px-2 py-0.5 rounded ${ic.used ? 'bg-slate-800 text-slate-500' : 'bg-emerald-950 text-emerald-400 border border-emerald-900/50'}`}>
                          {ic.used ? 'Đã dùng' : 'Khả dụng'}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              
              <div className="bg-slate-900 border border-slate-800 p-5 rounded-lg">
                <h3 className="text-cyan-500 font-bold mb-2">Hệ thống Đồ thị</h3>
                <p className="text-slate-400 text-sm italic leading-relaxed">
                  * Node xanh dương đại diện cho User. <br/>
                  * Node vàng đại diện cho lõi IndraCore.<br/>
                  * Edge kết nối là liên kết "Mời" giữa hai Node.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Nếu chưa đăng nhập, hiện Form
  return (
    <div className="min-h-screen bg-slate-950 text-white flex items-center justify-center font-sans p-4">
       <div className="w-full max-w-md bg-slate-900 border border-slate-800 p-8 rounded-xl shadow-2xl relative overflow-hidden">
         {/* Decorative Background Elements */}
         <div className="absolute top-0 right-0 -mr-8 -mt-8 w-32 h-32 rounded-full bg-cyan-900/20 blur-2xl"></div>
         <div className="absolute bottom-0 left-0 -ml-8 -mb-8 w-32 h-32 rounded-full bg-teal-900/20 blur-2xl"></div>
         
         <div className="relative z-10">
           <div className="text-center mb-8">
             <h1 className="text-2xl font-bold font-mono text-white tracking-widest uppercase mb-1">Indra<span className="text-cyan-400">Core</span></h1>
             <h2 className="text-sm font-semibold text-slate-400">
                {isLogin ? "ĐĂNG NHẬP" : "KHỞI TẠO NODE MỚI"}
             </h2>
           </div>

           <form onSubmit={handleSubmit} className="space-y-5">
             <div>
               <label className="block text-slate-400 text-sm mb-1 ml-1 font-medium">Tên đăng nhập</label>
               <input 
                 type="text" 
                 className="w-full bg-slate-900 text-white border border-slate-700 rounded-lg px-4 py-3 outline-none focus:border-cyan-500 hover:border-slate-600 transition-colors placeholder:text-slate-500"
                 value={form.username}
                 onChange={(e) => setForm({...form, username: e.target.value})}
                 required
                 placeholder="Nhập username"
               />
             </div>
             <div>
               <label className="block text-slate-400 text-sm mb-1 ml-1 font-medium">Mật khẩu</label>
               <input 
                 type="password" 
                 className="w-full bg-slate-900 text-white border border-slate-700 rounded-lg px-4 py-3 outline-none focus:border-cyan-500 hover:border-slate-600 transition-colors placeholder:text-slate-500"
                 value={form.password}
                 onChange={(e) => setForm({...form, password: e.target.value})}
                 required
                 placeholder="Nhập mật khẩu"
               />
             </div>
             {!isLogin && (
               <div>
                 <label className="block text-slate-400 text-sm mb-1 ml-1 font-medium">Mã mời (Tùy chọn)</label>
                 <input 
                   type="text" 
                   className="w-full bg-slate-900 text-white border border-slate-700 rounded-lg px-4 py-3 outline-none focus:border-cyan-500 hover:border-slate-600 transition-colors placeholder:text-slate-500 uppercase"
                   value={form.invite_code}
                   onChange={(e) => setForm({...form, invite_code: e.target.value.toUpperCase()})}
                   placeholder="Nhập mã mời liên kết (vd: INA-A5B2C3)"
                 />
               </div>
             )}
             <button 
               type="submit" 
               className="w-full bg-gradient-to-r from-cyan-600 to-teal-500 hover:from-cyan-500 hover:to-teal-400 text-white shadow-lg shadow-cyan-900/20 font-bold py-3 px-4 rounded-lg transition-all transform active:scale-[0.98]"
             >
               {isLogin ? "Vào Mạng Lưới" : "Tạo Node & Đăng Ký"}
             </button>
           </form>
           <div className="mt-6 text-center">
             <button 
               onClick={() => {
                 setIsLogin(!isLogin);
                 setForm({ username: '', password: '', invite_code: '' });
               }} 
               className="text-slate-400 hover:text-cyan-400 text-sm font-medium transition-colors"
             >
               {isLogin ? "Chưa có tài khoản? Khởi tạo node thay thế" : "Đã có tài khoản? Vào hệ thống"}
             </button>
           </div>
         </div>
       </div>
    </div>
  );
}

export default App;
