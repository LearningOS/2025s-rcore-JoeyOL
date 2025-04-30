+ 文件系统
  
  ![alt text](image.png)

  常规文件和目录都保存在持久存储设备中的，持久设备仅支持以扇区（或块）为单位的随机读写，这和我们预想的 **通过路径即可索引到文件并以字节流进行读写的用户视角** 有很大的不同，负责中间转换的便是文件系统，具体而言文件系统负责将逻辑上的目录树结构映射到持久存储设备上，决定设备上的每个扇区该存储哪些内容，同样文件系统也能够从持久存储设备还原逻辑上的目录树结构，但是一台计算机系统可能会有多个持久存储设备，它们上面的数据可能是以不同文件系统格式存储的，为了能够统一管理，内核中还需要再加一层中间层 **虚拟文件系统** 它规定了逻辑上目录树结构的通用格式以及相关操作的抽象接口，只要不同的底层文件系统均实现虚拟文件系统要求的哪些抽象接口，再加上挂载等方式，这些持久设备上的不同文件系统便可以用一个统一的逻辑目录树结构进行管理

+ easy-fs 文件系统
  
  easy-fs文件系统总共有五个层次：
  1. 磁盘块管理设备接口层：定义了以块大小为单位对磁盘读写的接口
  2. 块缓存层：在内存中缓存磁盘块的数据
  3. 磁盘数据结构层：磁盘上超级块、位图、索引节点、数据块、目录项等核心数据结构和相关处理
  4. 磁盘块管理器层：磁盘块的分配和回收处理
  5. 索引节点层：管理索引节点数据结构，实现文件创建/文件打开/文件读写等成员函数

+ 块设备接口层
  
  定义设备驱动需要实现的块读写接口：

    ```Rust
    // easy-fs/src/block_dev.rs

    pub trait BlockDevice : Send + Sync + Any {
        fn read_block(&self, block_id: usize, buf: &mut [u8]);
        fn write_block(&self, block_id: usize, buf: &[u8]);
    }
    ```

  体现了easy-fs的泛用性，可以访问所有实现了这个Trait的块设备驱动程序

+ 块缓存
  
  块缓存定义如下：

    ```Rust
    // easy-fs/src/lib.rs

    pub const BLOCK_SZ: usize = 512;

    // easy-fs/src/block_cache.rs

    pub struct BlockCache {
        cache: [u8; BLOCK_SZ],
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
        modified: bool,
    }
    ```

  一旦磁盘块以已经存在于内存缓存中，CPU就可以直接访问磁盘块数据了：

    ```Rust
    // easy-fs/src/block_cache.rs

    impl BlockCache {
        fn addr_of_offset(&self, offset: usize) -> usize {
            &self.cache[offset] as *const _ as usize
        }

        pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
            let type_size = core::mem::size_of::<T>();
            assert!(offset + type_size <= BLOCK_SZ);
            let addr = self.addr_of_offset(offset);
            unsafe { &*(addr as *const T) }
        }

        pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
            let type_size = core::mem::size_of::<T>();
            assert!(offset + type_size <= BLOCK_SZ);
            self.modified = true;
            let addr = self.addr_of_offset(offset);
            unsafe { &mut *(addr as *mut T) }
        }
    }
    // easy-fs/src/block_cache.rs

    impl BlockCache {
        pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
            f(self.get_ref(offset))
        }

        pub fn modify<T, V>(&mut self, offset:usize, f: impl FnOnce(&mut T) -> V) -> V {
            f(self.get_mut(offset))
        }
    }
    ```
  
  显然内存中同时只能驻留有限个磁盘块的缓冲区，因此我们还需要实现一个块缓存全局管理器：
  
    ```Rust
    // easy-fs/src/block_cache.rs

    impl BlockCacheManager {
        pub fn get_block_cache(
            &mut self,
            block_id: usize,
            block_device: Arc<dyn BlockDevice>,
        ) -> Arc<Mutex<BlockCache>> {
            if let Some(pair) = self.queue
                .iter()
                .find(|pair| pair.0 == block_id) {
                    Arc::clone(&pair.1)
            } else {
                // substitute
                if self.queue.len() == BLOCK_CACHE_SIZE {
                    // from front to tail
                    if let Some((idx, _)) = self.queue
                        .iter()
                        .enumerate()
                        .find(|(_, pair)| Arc::strong_count(&pair.1) == 1) {
                        self.queue.drain(idx..=idx);
                    } else {
                        panic!("Run out of BlockCache!");
                    }
                }
                // load block into mem and push back
                let block_cache = Arc::new(Mutex::new(
                    BlockCache::new(block_id, Arc::clone(&block_device))
                ));
                self.queue.push_back((block_id, Arc::clone(&block_cache)));
                block_cache
            }
        }
    }
    ```

  实现了一个简单的FIFO缓存替换算法

+ 磁盘布局以及磁盘上的数据结构
  
  easy-fs的磁盘布局如下：

  ![alt text](image-1.png)

  定义各数据结构如下：

  + 超级块：
    
    ```Rust
    // easy-fs/src/layout.rs

    #[repr(C)]
    pub struct SuperBlock {
        magic: u32,
        pub total_blocks: u32,
        pub inode_bitmap_blocks: u32,
        pub inode_area_blocks: u32,
        pub data_bitmap_blocks: u32,
        pub data_area_blocks: u32,
    }
    ```

  + 位图：
    
    ```Rust
    // easy-fs/src/bitmap.rs

    pub struct Bitmap {
        start_block_id: usize,
        blocks: usize,
    }

    impl Bitmap {
        pub fn new(start_block_id: usize, blocks: usize) -> Self {
            Self {
                start_block_id,
                blocks,
            }
        }
    }

    // easy-fs/src/bitmap.rs
    type BitmapBlock = [u64; 64];
    ```

    位图中仅保存了它所在区域的起始块编号以及区域的长度为多少个块，这是一个内存数据结构，对应的磁盘数据结构 `BitmaoBlock` 是一个长度为64的一个 `u64` 数组，整个数组包含 $64\times64=4096\ \text{bits}$，因此能够管理的块一共有4096个

    Rust语法补充：闭包是持有外部环境变量的函数。所谓外部环境, 就是指创建闭包时所在的词法作用域。Rust中定义的闭包，按照对外部环境变量的使用方式（借用、复制、转移所有权），分为三个类型: Fn、FnMut、FnOnce。Fn类型的闭包会在闭包内部以共享借用的方式使用环境变量；FnMut类型的闭包会在闭包内部以独占借用的方式使用环境变量；而FnOnce类型的闭包会在闭包内部以所有者的身份使用环境变量。由此可见，根据闭包内使用环境变量的方式，即可判断创建出来的闭包的类型

  + 索引节点

    在磁盘上的索引节点区域每个块上都有若干个索引节点 `DiskInode`：

    ```Rust
    // easy-fs/src/layout.rs

    const INODE_DIRECT_COUNT: usize = 28;

    #[repr(C)]
    pub struct DiskInode {
        pub size: u32,
        pub direct: [u32; INODE_DIRECT_COUNT],
        pub indirect1: u32,
        pub indirect2: u32,
        type_: DiskInodeType,
    }

    #[derive(PartialEq)]
    pub enum DiskInodeType {
        File,
        Directory,
    }
    ```

    索引的方式一共有三种，**直接索引、一级索引、二级索引**，直接索引指向的数据块存在数据，一、二级索引指向的数据以 `u32` 的格式存储着下一级索引的数据块编号

  + 数据块和目录项
  
    对于一个文件而言，最后索引到的数据块是一个字节序列，对于一个目录而言，最后索引到的数据是一个目录项数组：

    ```Rust
    // easy-fs/src/layout.rs

    type DataBlock = [u8; BLOCK_SZ];
    // easy-fs/src/layout.rs

    const NAME_LENGTH_LIMIT: usize = 27;

    #[repr(C)]
    pub struct DirEntry {
        name: [u8; NAME_LENGTH_LIMIT + 1],
        inode_number: u32,
    }

    pub const DIRENT_SZ: usize = 32;
    ```

+ 磁盘块管理器
  
  从这一层开始，所有的数据结构都在内存：

    ```Rust
    // easy-fs/src/efs.rs

    pub struct EasyFileSystem {
        pub block_device: Arc<dyn BlockDevice>,
        pub inode_bitmap: Bitmap,
        pub data_bitmap: Bitmap,
        inode_area_start_block: u32,
        data_area_start_block: u32,
    }
    ```

  `EasyFileSystem`包含索引节点和数据块的两个位图`inode_bitmap`和`data_bitmap`，还记录下索引节点区域和数据块区域起始块编号方便确定每个索引节点和数据块在磁盘上的具体位置。我们还要在其中保留块设备的一个指针`block_device`，在进行后续操作的时候，该指针会被拷贝并传递给下层的数据结构，让它们也能够直接访问块设备

  通过`create`方法可以在块设备上创建并初始化一个 easy-fs 文件系统：

    ```Rust
    // easy-fs/src/efs.rs

    impl EasyFileSystem {
        pub fn create(
            block_device: Arc<dyn BlockDevice>,
            total_blocks: u32,
            inode_bitmap_blocks: u32,
        ) -> Arc<Mutex<Self>> {
            // calculate block size of areas & create bitmaps
            let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
            let inode_num = inode_bitmap.maximum();
            let inode_area_blocks =
                ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
            let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
            let data_total_blocks = total_blocks - 1 - inode_total_blocks;
            let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
            let data_area_blocks = data_total_blocks - data_bitmap_blocks;
            let data_bitmap = Bitmap::new(
                (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
                data_bitmap_blocks as usize,
            );
            let mut efs = Self {
                block_device: Arc::clone(&block_device),
                inode_bitmap,
                data_bitmap,
                inode_area_start_block: 1 + inode_bitmap_blocks,
                data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
            };
            // clear all blocks
            for i in 0..total_blocks {
                get_block_cache(
                    i as usize,
                    Arc::clone(&block_device)
                )
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() { *byte = 0; }
                });
            }
            // initialize SuperBlock
            get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .modify(0, |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            });
            // write back immediately
            // create a inode for root node "/"
            assert_eq!(efs.alloc_inode(), 0);
            let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
            get_block_cache(
                root_inode_block_id as usize,
                Arc::clone(&block_device)
            )
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
            Arc::new(Mutex::new(efs))
        }
    }
    ```

+ 索引节点
  
  `EasyFileSystem`实现了磁盘布局并能够将磁盘块有效的管理起来。但是对于文件系统的使用者而言，他们往往不关心磁盘布局是如何实现的，而是更希望能够直接看到目录树结构中逻辑上的文件和目录。为此需要设计索引节点`Inode`暴露给文件系统的使用者，让他们能够直接对文件和目录进行操作。`Inode`和`DiskInode`的区别从它们的名字中就可以看出：`DiskInode`放在磁盘块中比较固定的位置，而`Inode`是放在内存中的记录文件索引节点信息的数据结构 

    ```Rust
    // easy-fs/src/vfs.rs

    pub struct Inode {
        block_id: usize,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    }
    ```

  仿照`BlockCache::read/modify`，我们可以设计两个方法来简化对于`Inode`对应的磁盘上的`DiskInode`的访问流程，而不是每次都需要`get_block_cache.lock.read/modify`：

    ```Rust
    // easy-fs/src/vfs.rs

    impl Inode {
        fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
            get_block_cache(
                self.block_id,
                Arc::clone(&self.block_device)
            ).lock().read(self.block_offset, f)
        }

        fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
            get_block_cache(
                self.block_id,
                Arc::clone(&self.block_device)
            ).lock().modify(self.block_offset, f)
        }
    }
    ```
  