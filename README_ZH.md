# VisioGrid

## 項目介紹
VisioGrid 是一個使用 Rust 開發的分布式計算平台，專注於圖像辨識。該項目旨在建立一個高效的分布式系統，用於處理大規模圖像資料。通過在多代理環境中並行處理任務，VisioGrid 提高了圖像處理的效率和速度。

## 主要特性
- **代理通訊**：使用 TcpStream 實現代理之間的穩定通訊。
- **代理監控**：實時監控每個代理以確保穩定運行。
- **任務和文件傳輸**：高效傳輸任務信息和文件。
- **圖像識別**：支持多種圖像辨識模型。

## 技術棧
- **Rust**：充分利用 Rust 的高性能和安全特性。
- **Actix Web**：提供用戶友好的 Web 管理介面。
- **tokio**：使用 tokio 的異步運行時進行性能優化。
- **tch-rs**：使用 Torch 的 Rust 綁定進行深度學習模型的推理。
- **GStreamer**：用於處理媒體流和媒體內容。

## 安裝和配置
- 複製倉庫：`git clone https://github.com/DaLaw2/VisioGrid`
- 從原碼編譯：
    - 编譯管理節點（Management）需要安裝 GStreamer：`cargo build --release --bin Management`
    - 编譯代理（Agent）需要安裝 LibTorch：`cargo build --release --bin Agent`
- 使用 Docker 運行：
    - 管理節點（Management）和代理（Agent）的 Docker 容器已包含所有必要依賴，無需手動安裝。

## 如何使用
- 根據需求從源代碼編譯或使用 Docker 容器運行管理節點和代理。
- 使用 Docker 容器運行時，構建並運行容器指令如下：
    - 構建管理節點容器：`docker build -t management-image ./Docker/Management`
    - 運行管理節點容器：`docker run management-image`
    - 構建代理容器：`docker build -t agent-image ./Docker/Agent`
    - 運行代理容器：`docker run agent-image`
- 容器運行時不需要輸入任何命令，會根據初始化設置自動配置。

## 錯誤處理和安全
- 主動處理所有錯誤，並通過日誌增強系統的穩定性和安全性。
